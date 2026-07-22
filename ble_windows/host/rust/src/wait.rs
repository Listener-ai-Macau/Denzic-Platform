//! Blocking waits for WinRT `IAsyncOperation` with timeout and optional
//! cooperative cancellation.
//!
//! Long Windows BLE operations outlive the product-level state that spawned
//! them (a background capture may need the GATT path back immediately), so
//! every wait accepts an optional [`BleCancel`] token. The token is checked
//! before the operation status on every poll; a pre-set token fails the wait
//! immediately and cancels the underlying operation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCommunicationStatus, GattWriteResult,
};
use windows::Foundation::{AsyncStatus, IAsyncOperation};

use crate::ASYNC_POLL_INTERVAL_MS;

/// Cooperative cancellation token for the blocking wait helpers.
#[derive(Debug, Clone)]
pub struct BleCancel {
    token: Option<Arc<AtomicBool>>,
    canceller: &'static str,
}

impl BleCancel {
    /// No cancellation: waits run until completion, error, or timeout.
    pub const NONE: Self = Self {
        token: None,
        canceller: "background operation",
    };

    pub fn new(token: Arc<AtomicBool>, canceller: &'static str) -> Self {
        Self {
            token: Some(token),
            canceller,
        }
    }

    pub fn requested(&self) -> bool {
        self.token
            .as_ref()
            .is_some_and(|token| token.load(Ordering::SeqCst))
    }

    pub fn cancelled_error(&self, label: &str) -> String {
        format!("BLE {label} cancelled by {}", self.canceller)
    }
}

impl Default for BleCancel {
    fn default() -> Self {
        Self::NONE
    }
}

/// Wait for an `IAsyncOperation` without cancellation.
pub fn wait_async_operation<T: windows::core::RuntimeType>(
    operation: IAsyncOperation<T>,
    timeout: Duration,
    label: &str,
) -> Result<T, String> {
    wait_async_operation_with_cancel(operation, timeout, label, &BleCancel::NONE)
}

/// Wait for an `IAsyncOperation`, polling status until it completes, errors,
/// times out, or the cancel token is set (in which case the operation is
/// cancelled and closed before returning).
pub fn wait_async_operation_with_cancel<T: windows::core::RuntimeType>(
    operation: IAsyncOperation<T>,
    timeout: Duration,
    label: &str,
    cancel: &BleCancel,
) -> Result<T, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if cancel.requested() {
            let _ = operation.Cancel();
            let _ = operation.Close();
            return Err(cancel.cancelled_error(label));
        }
        match operation
            .Status()
            .map_err(|err| format!("BLE {label} async status failed: {err}"))?
        {
            AsyncStatus::Completed => {
                return operation
                    .GetResults()
                    .map_err(|err| format!("BLE {label} async result failed: {err}"));
            }
            AsyncStatus::Error => {
                let code = operation.ErrorCode().ok();
                let _ = operation.Close();
                return Err(format!("BLE {label} async error: {code:?}"));
            }
            AsyncStatus::Canceled => {
                let _ = operation.Close();
                return Err(format!("BLE {label} async canceled"));
            }
            AsyncStatus::Started => {
                if Instant::now() >= deadline {
                    let _ = operation.Cancel();
                    let _ = operation.Close();
                    return Err(format!(
                        "BLE {label} timed out after {} ms",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(ASYNC_POLL_INTERVAL_MS));
            }
            status => {
                let _ = operation.Close();
                return Err(format!("BLE {label} unknown async status={status:?}"));
            }
        }
    }
}

/// Method-style variant of [`wait_async_operation`] for `IAsyncOperation<T>`.
pub trait AsyncOperationTimeoutExt<T: windows::core::RuntimeType> {
    fn wait_ble_result(self, timeout: Duration, label: &str) -> Result<T, String>;
    fn wait_ble_result_with_cancel(
        self,
        timeout: Duration,
        label: &str,
        cancel: &BleCancel,
    ) -> Result<T, String>;
}

impl<T: windows::core::RuntimeType> AsyncOperationTimeoutExt<T> for IAsyncOperation<T> {
    fn wait_ble_result(self, timeout: Duration, label: &str) -> Result<T, String> {
        wait_async_operation(self, timeout, label)
    }

    fn wait_ble_result_with_cancel(
        self,
        timeout: Duration,
        label: &str,
        cancel: &BleCancel,
    ) -> Result<T, String> {
        wait_async_operation_with_cancel(self, timeout, label, cancel)
    }
}

/// Wait for a GATT write-with-result operation (no cancellation; GATT writes
/// are short and bounded by `timeout`).
pub fn wait_gatt_write_result(
    operation: IAsyncOperation<GattWriteResult>,
    timeout: Duration,
    label: &str,
) -> Result<GattWriteResult, String> {
    let deadline = Instant::now() + timeout;
    loop {
        match operation
            .Status()
            .map_err(|err| format!("BLE {label} write async status failed: {err}"))?
        {
            AsyncStatus::Completed => {
                return operation
                    .GetResults()
                    .map_err(|err| format!("BLE {label} write result failed: {err}"));
            }
            AsyncStatus::Error => {
                let code = operation.ErrorCode().ok();
                let _ = operation.Close();
                return Err(format!("BLE {label} write async error: {code:?}"));
            }
            AsyncStatus::Canceled => {
                let _ = operation.Close();
                return Err(format!("BLE {label} write async canceled"));
            }
            AsyncStatus::Started => {
                if Instant::now() >= deadline {
                    let _ = operation.Cancel();
                    let _ = operation.Close();
                    return Err(format!(
                        "BLE {label} write timed out after {} ms",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(ASYNC_POLL_INTERVAL_MS));
            }
            status => {
                let _ = operation.Close();
                return Err(format!("BLE {label} write unknown async status={status:?}"));
            }
        }
    }
}

/// Wait for a GATT write-without-result operation (no cancellation).
pub fn wait_gatt_communication_status(
    operation: IAsyncOperation<GattCommunicationStatus>,
    timeout: Duration,
    label: &str,
) -> Result<GattCommunicationStatus, String> {
    let deadline = Instant::now() + timeout;
    loop {
        match operation
            .Status()
            .map_err(|err| format!("BLE {label} write async status failed: {err}"))?
        {
            AsyncStatus::Completed => {
                return operation
                    .GetResults()
                    .map_err(|err| format!("BLE {label} write result failed: {err}"));
            }
            AsyncStatus::Error => {
                let code = operation.ErrorCode().ok();
                let _ = operation.Close();
                return Err(format!("BLE {label} write async error: {code:?}"));
            }
            AsyncStatus::Canceled => {
                let _ = operation.Close();
                return Err(format!("BLE {label} write async canceled"));
            }
            AsyncStatus::Started => {
                if Instant::now() >= deadline {
                    let _ = operation.Cancel();
                    let _ = operation.Close();
                    return Err(format!(
                        "BLE {label} write timed out after {} ms",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(ASYNC_POLL_INTERVAL_MS));
            }
            status => {
                let _ = operation.Close();
                return Err(format!("BLE {label} write unknown async status={status:?}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Devices::Bluetooth::BluetoothLEDevice;

    #[test]
    fn preset_cancel_token_interrupts_wait_before_completion() {
        let operation = match BluetoothLEDevice::FromBluetoothAddressAsync(0x0000_A1B2_C3D4) {
            Ok(operation) => operation,
            Err(_) => return, // WinRT Bluetooth unavailable on this host
        };
        let cancel = BleCancel::new(Arc::new(AtomicBool::new(true)), "test recovery");
        let err = wait_async_operation_with_cancel(
            operation,
            Duration::from_secs(30),
            "cancel probe",
            &cancel,
        )
        .expect_err("pre-set cancel token must fail the wait");
        assert!(
            err.contains("BLE cancel probe cancelled by test recovery"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn wait_without_completion_eventually_fails() {
        let operation = match BluetoothLEDevice::FromBluetoothAddressAsync(0x0000_A1B2_C3D4) {
            Ok(operation) => operation,
            Err(_) => return, // WinRT Bluetooth unavailable on this host
        };
        let result = wait_async_operation(operation, Duration::from_millis(250), "timeout probe");
        assert!(
            result.is_err(),
            "bogus address must not complete successfully"
        );
    }

    #[test]
    fn cancel_token_reports_requested_flag() {
        let token = Arc::new(AtomicBool::new(false));
        let cancel = BleCancel::new(Arc::clone(&token), "test recovery");
        assert!(!cancel.requested());
        token.store(true, Ordering::SeqCst);
        assert!(cancel.requested());
        assert!(!BleCancel::NONE.requested());
    }
}
