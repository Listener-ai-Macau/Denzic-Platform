//! Close locally owned BLE objects on every exit, including failed discovery.
//! Dropping a COM reference alone does not call WinRT IClosable::Close.
//! This releases an application's ownership; it does not promise a physical
//! disconnect when another application still owns the connection.
use std::ops::Deref;

pub trait ReleaseBleResource {
    const LABEL: &'static str;
    fn release(&self) -> Result<(), String>;
}

/// Own immediately after acquisition so a later `?` cannot skip cleanup.
/// No radio operation, allocation, negotiation or wait occurs on acquisition.
pub struct ScopedBleResource<T: ReleaseBleResource> { value: Option<T> }

impl<T: ReleaseBleResource> ScopedBleResource<T> {
    pub fn new(value: T) -> Self { Self { value: Some(value) } }
    /// Retire once even if Close reports an error; later Drop cannot retry an
    /// invalid/closed object. Callers can retain that unconfirmed result.
    pub fn close(&mut self) -> Result<(), String> {
        match self.value.take() { Some(value) => value.release(), None => Ok(()) }
    }
}
impl<T: ReleaseBleResource> Deref for ScopedBleResource<T> {
    type Target = T;
    fn deref(&self) -> &T { self.value.as_ref().expect("BLE resource already retired") }
}
impl<T: ReleaseBleResource> Drop for ScopedBleResource<T> {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // Cleanup never replaces the real transfer/preflight failure.
            eprintln!("BLE owned_resource label={} release_failed={error} physical_disconnect=unconfirmed", T::LABEL);
        }
    }
}

#[cfg(target_os="windows")]
mod winrt {
    use super::ReleaseBleResource;
    use windows::Devices::Bluetooth::{BluetoothLEDevice, BluetoothLEPreferredConnectionParametersRequest};
    use windows::Devices::Bluetooth::GenericAttributeProfile::{GattDeviceService, GattSession};
    impl ReleaseBleResource for GattSession {
        const LABEL: &'static str = "GattSession";
        fn release(&self) -> Result<(),String> {
            // Attempt Close even if the disconnected session refuses the
            // property write. Neither result proves radio disconnection.
            let maintain=self.SetMaintainConnection(false);
            let close=self.Close();
            match (maintain,close) {
                (Ok(()),Ok(())) => Ok(()),
                (a,b) => Err(format!("maintain_off={a:?}; close={b:?}")),
            }
        }
    }
    macro_rules! closable {
        ($ty:ty,$label:literal) => { impl ReleaseBleResource for $ty {
            const LABEL: &'static str = $label;
            fn release(&self) -> Result<(),String> { self.Close().map_err(|e|e.to_string()) }
        }};
    }
    closable!(BluetoothLEPreferredConnectionParametersRequest,"PreferredConnectionRequest");
    closable!(GattDeviceService,"GattDeviceService");
    closable!(BluetoothLEDevice,"BluetoothLEDevice");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc,Mutex};
    struct Object { id:usize, calls:Arc<Mutex<Vec<usize>>>, fails:bool }
    impl ReleaseBleResource for Object {
        const LABEL:&'static str="fake-owned-object";
        fn release(&self)->Result<(),String> {
            self.calls.lock().unwrap().push(self.id);
            if self.fails {Err("object already unavailable".into())} else {Ok(())}
        }
    }
    fn object(id:usize,calls:&Arc<Mutex<Vec<usize>>>,fails:bool)->Object {
        Object {id,calls:calls.clone(),fails}
    }
    fn preflight(acquired:usize,calls:&Arc<Mutex<Vec<usize>>>)->Result<(),&'static str> {
        let _device=ScopedBleResource::new(object(0,calls,false));
        if acquired==1 {return Err("service discovery failed");}
        let _service=ScopedBleResource::new(object(1,calls,false));
        if acquired==2 {return Err("access denied");}
        let _session=ScopedBleResource::new(object(2,calls,false));
        if acquired==3 {return Err("characteristic missing");}
        let _preference=ScopedBleResource::new(object(3,calls,false));
        if acquired==4 {return Err("manifest rejected");}
        Ok(())
    }
    #[test]
    fn every_preflight_exit_closes_acquired_objects_in_reverse_order() {
        // Compare COM-reference-only destruction with explicit IClosable
        // ownership, including errors before a transfer object even exists.
        let raw=Arc::new(Mutex::new(vec![]));
        { let _raw_object=object(0,&raw,false); }
        assert!(raw.lock().unwrap().is_empty());
        for acquired in 1..=5 {
            let calls=Arc::new(Mutex::new(vec![]));
            assert_eq!(preflight(acquired,&calls).is_ok(),acquired==5);
            let count=acquired.min(4);
            assert_eq!(*calls.lock().unwrap(),(0..count).rev().collect::<Vec<_>>());
        }
    }
    #[test]
    fn explicit_failed_close_is_not_repeated_on_drop() {
        let calls=Arc::new(Mutex::new(vec![]));
        {
            let mut resource=ScopedBleResource::new(object(9,&calls,true));
            assert!(resource.close().is_err());
            assert!(resource.close().is_ok());
        }
        assert_eq!(*calls.lock().unwrap(),[9]);
    }
    #[test]
    fn one_cleanup_failure_preserves_primary_error_and_releases_other_owners() {
        fn failed(calls:&Arc<Mutex<Vec<usize>>>)->Result<(),&'static str> {
            let _device=ScopedBleResource::new(object(0,calls,false));
            let _service=ScopedBleResource::new(object(1,calls,false));
            let _session=ScopedBleResource::new(object(2,calls,true));
            let _preference=ScopedBleResource::new(object(3,calls,false));
            Err("firmware write failed")
        }
        let calls=Arc::new(Mutex::new(vec![]));
        assert_eq!(failed(&calls),Err("firmware write failed"));
        assert_eq!(*calls.lock().unwrap(),[3,2,1,0]);
    }
}
