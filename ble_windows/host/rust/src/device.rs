//! Opening `BluetoothLEDevice` handles by 48-bit address, with timeout,
//! optional random-identity addressing, and a reopen-by-device-id fallback.
//!
//! `use_random_identity` is decided by the caller (products that track
//! native Windows HID pairing know when an address is a random resolvable
//! identity); this module only maps the flag onto the right WinRT open call.

use std::time::Duration;

use windows::core::HSTRING;
use windows::Devices::Bluetooth::{BluetoothAddressType, BluetoothLEDevice};

use crate::wait::{wait_async_operation_with_cancel, BleCancel};
use crate::DEFAULT_DISCOVERY_TIMEOUT_MS;

pub fn open_ble_device(
    address: u64,
    use_random_identity: bool,
    cancel: &BleCancel,
) -> Result<BluetoothLEDevice, String> {
    open_ble_device_with_timeout(
        address,
        Duration::from_millis(DEFAULT_DISCOVERY_TIMEOUT_MS),
        use_random_identity,
        cancel,
    )
}

pub fn open_ble_device_by_address(
    address: u64,
    use_random_identity: bool,
    cancel: &BleCancel,
) -> Result<BluetoothLEDevice, String> {
    open_ble_device_by_address_with_timeout(
        address,
        Duration::from_millis(DEFAULT_DISCOVERY_TIMEOUT_MS),
        use_random_identity,
        cancel,
    )
}

/// Open by address, then reopen through the device id when available (the
/// id-based handle tracks renames and cache refreshes better).
pub fn open_ble_device_with_timeout(
    address: u64,
    timeout: Duration,
    use_random_identity: bool,
    cancel: &BleCancel,
) -> Result<BluetoothLEDevice, String> {
    let device =
        open_ble_device_by_address_with_timeout(address, timeout, use_random_identity, cancel)?;

    let device_id = device
        .DeviceId()
        .map(|id| id.to_string_lossy())
        .unwrap_or_default();
    if device_id.is_empty() {
        return Ok(device);
    }

    match BluetoothLEDevice::FromIdAsync(&HSTRING::from(device_id.as_str()))
        .ok()
        .and_then(|op| {
            wait_async_operation_with_cancel(op, timeout, "device open by id", cancel).ok()
        }) {
        Some(device_by_id) => {
            let _ = device.Close();
            Ok(device_by_id)
        }
        None => {
            log::warn!("[ble-windows] BLE device reopen by id failed, using address handle");
            Ok(device)
        }
    }
}

pub fn open_ble_device_by_address_with_timeout(
    address: u64,
    timeout: Duration,
    use_random_identity: bool,
    cancel: &BleCancel,
) -> Result<BluetoothLEDevice, String> {
    let operation = if use_random_identity {
        log::debug!("[ble-windows] opening address={address:012X} as a random BLE identity");
        BluetoothLEDevice::FromBluetoothAddressWithBluetoothAddressTypeAsync(
            address,
            BluetoothAddressType::Random,
        )
    } else {
        BluetoothLEDevice::FromBluetoothAddressAsync(address)
    };
    operation
        .map_err(|err| format!("BLE device open by address failed: {err}"))
        .and_then(|op| {
            wait_async_operation_with_cancel(op, timeout, "device open by address", cancel)
        })
}
