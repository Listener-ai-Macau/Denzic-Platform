//! Advertisement scanning and Windows device-enumeration helpers.
//!
//! Filters (target name, service UUID) are always caller-supplied. Includes
//! the generic `DeviceInformation` property accessors, the AEP selector
//! strings, Swift Pair manufacturer-data decoding, and service/paired-device
//! address discovery used to locate a named BLE target.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::core::{Interface, GUID, HSTRING};
use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisement, BluetoothLEAdvertisementReceivedEventArgs,
    BluetoothLEAdvertisementWatcher, BluetoothLEScanningMode,
};
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::GattDeviceService;
use windows::Devices::Enumeration::DeviceInformation;
use windows::Foundation::{IPropertyValue, TypedEventHandler};

use crate::address::{
    ble_advertisement_name_matches, bluetooth_name_matches_expected, hex_bytes,
    parse_bluetooth_address_from_device_id, parse_bluetooth_address_hex, push_unique_address,
    swift_pair_display_name_from_manufacturer_entry,
};
use crate::gatt::buffer_to_vec;
use crate::wait::{wait_async_operation_with_cancel, BleCancel};

/// AQS selector for BLE AEP (Association Endpoint) devices.
pub const WINDOWS_BLE_AEP_SELECTOR: &str =
    "(System.Devices.Aep.ProtocolId:=\"{bb7bb05e-5972-42b5-94fc-76eaa7084d49}\")";
/// AQS selector for connectable BLE AEP devices.
pub const WINDOWS_BLE_AEP_CONNECTABLE_SELECTOR: &str =
    "(System.Devices.Aep.ProtocolId:=\"{bb7bb05e-5972-42b5-94fc-76eaa7084d49}\") AND (System.Devices.Aep.Bluetooth.Le.IsConnectable:=System.StructuredQueryType.Boolean#True)";
pub const WINDOWS_AEP_DEVICE_ADDRESS_PROPERTY: &str = "System.Devices.Aep.DeviceAddress";
pub const WINDOWS_AEP_IS_PAIRED_PROPERTY: &str = "System.Devices.Aep.IsPaired";
pub const WINDOWS_AEP_IS_CONNECTED_PROPERTY: &str = "System.Devices.Aep.IsConnected";
pub const WINDOWS_AEP_IS_PRESENT_PROPERTY: &str = "System.Devices.Aep.IsPresent";
pub const WINDOWS_AEP_BLE_IS_CONNECTABLE_PROPERTY: &str =
    "System.Devices.Aep.Bluetooth.Le.IsConnectable";
pub const WINDOWS_ITEM_NAME_DISPLAY_PROPERTY: &str = "System.ItemNameDisplay";

pub fn device_information_display_name(info: &DeviceInformation) -> String {
    info.Name()
        .map(|value| value.to_string_lossy())
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| device_information_property_string(info, WINDOWS_ITEM_NAME_DISPLAY_PROPERTY))
        .unwrap_or_default()
}

pub fn device_information_bluetooth_address(info: &DeviceInformation) -> Option<u64> {
    device_information_property_string(info, WINDOWS_AEP_DEVICE_ADDRESS_PROPERTY)
        .as_deref()
        .and_then(parse_bluetooth_address_hex)
}

pub fn device_information_property_string(info: &DeviceInformation, key: &str) -> Option<String> {
    let properties = info.Properties().ok()?;
    let key = HSTRING::from(key);
    if !properties.HasKey(&key).ok()? {
        return None;
    }
    let value = properties.Lookup(&key).ok()?;
    let value = value.cast::<IPropertyValue>().ok()?;
    value.GetString().ok().map(|value| value.to_string_lossy())
}

pub fn device_information_property_bool(info: &DeviceInformation, key: &str) -> Option<bool> {
    let properties = info.Properties().ok()?;
    let key = HSTRING::from(key);
    if !properties.HasKey(&key).ok()? {
        return None;
    }
    let value = properties.Lookup(&key).ok()?;
    let value = value.cast::<IPropertyValue>().ok()?;
    value.GetBoolean().ok()
}

/// Actively scan advertisements for up to `timeout`, returning the addresses
/// whose local name matches `expected_name` (trimmed, case-insensitive).
pub fn scan_ble_advertisements_by_name(
    context: &str,
    expected_name: &str,
    timeout: Duration,
) -> Result<Vec<u64>, String> {
    let watcher = BluetoothLEAdvertisementWatcher::new()
        .map_err(|err| format!("{context} advertisement watcher create failed: {err}"))?;
    watcher
        .SetScanningMode(BluetoothLEScanningMode::Active)
        .map_err(|err| format!("{context} advertisement active scan failed: {err}"))?;

    let (tx, rx) = mpsc::channel::<(u64, String, i16)>();
    let expected_name_for_handler = expected_name.to_string();
    let handler = TypedEventHandler::<
        BluetoothLEAdvertisementWatcher,
        BluetoothLEAdvertisementReceivedEventArgs,
    >::new(move |_watcher, args| {
        let Some(args) = args.as_ref() else {
            return Ok(());
        };
        let Ok(advertisement) = args.Advertisement() else {
            return Ok(());
        };
        let name = advertisement
            .LocalName()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        if !ble_advertisement_name_matches(&name, &expected_name_for_handler) {
            return Ok(());
        }
        let address = args.BluetoothAddress().unwrap_or_default();
        if address == 0 {
            return Ok(());
        }
        let rssi = args.RawSignalStrengthInDBm().unwrap_or_default();
        let _ = tx.send((address, name, rssi));
        Ok(())
    });

    let token = watcher
        .Received(&handler)
        .map_err(|err| format!("{context} advertisement handler failed: {err}"))?;
    watcher
        .Start()
        .map_err(|err| format!("{context} advertisement scan start failed: {err}"))?;

    let deadline = Instant::now() + timeout;
    let mut addresses = Vec::new();
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = remaining.min(Duration::from_millis(500));
        match rx.recv_timeout(timeout) {
            Ok((address, name, rssi)) => {
                if addresses.contains(&address) {
                    continue;
                }
                log::info!(
                    "[ble-windows] {context} advertisement candidate name={name} address={address:012X} rssi={rssi}"
                );
                addresses.push(address);
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = watcher.Stop();
    let _ = watcher.RemoveReceived(token);

    if addresses.is_empty() {
        return Err(format!(
            "no Bluetooth advertisement named {expected_name:?} seen for {context} in {} ms",
            timeout.as_millis()
        ));
    }
    Ok(addresses)
}

/// Find the address of a device advertising `service_uuid` whose Windows
/// display name matches `target_name`. When exactly one device exposes the
/// service, its address is returned even if the Windows name cache lags.
pub fn find_bluetooth_target_service_address(
    service_uuid: GUID,
    target_name: &str,
    timeout: Duration,
    context: &str,
    cancel: &BleCancel,
) -> Option<u64> {
    let selector = GattDeviceService::GetDeviceSelectorFromUuid(service_uuid).ok()?;
    let operation = DeviceInformation::FindAllAsyncAqsFilter(&selector).ok()?;
    let services = wait_async_operation_with_cancel(operation, timeout, context, cancel).ok()?;
    let mut service_addresses = Vec::new();
    for index in 0..services.Size().ok()? {
        let info = services.GetAt(index).ok()?;
        let name = info
            .Name()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        let id = info.Id().ok()?.to_string_lossy();
        if let Some(address) = parse_bluetooth_address_from_device_id(&id) {
            push_unique_address(&mut service_addresses, address);
        }
        if !bluetooth_name_matches_expected(&name, target_name) {
            continue;
        }
        if let Some(address) = parse_bluetooth_address_from_device_id(&id) {
            log::info!(
                "[ble-windows] {context}: learned target address from service target={target_name:?} name={name:?} address={address:012X}"
            );
            return Some(address);
        }
    }
    if service_addresses.len() == 1 {
        let address = service_addresses[0];
        log::info!(
            "[ble-windows] {context}: learned sole service address despite Windows name cache mismatch target={target_name:?} address={address:012X}"
        );
        return Some(address);
    }
    None
}

/// Find the address of an already-paired BLE device whose Windows display
/// name matches `target_name`.
pub fn find_paired_bluetooth_target_address(
    target_name: &str,
    timeout: Duration,
    context: &str,
    cancel: &BleCancel,
) -> Option<u64> {
    let selector = BluetoothLEDevice::GetDeviceSelectorFromPairingState(true).ok()?;
    let operation = DeviceInformation::FindAllAsyncAqsFilter(&selector).ok()?;
    let devices = wait_async_operation_with_cancel(operation, timeout, context, cancel).ok()?;
    for index in 0..devices.Size().ok()? {
        let info = devices.GetAt(index).ok()?;
        let name = info
            .Name()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        if !bluetooth_name_matches_expected(&name, target_name) {
            continue;
        }
        let id = info.Id().ok()?.to_string_lossy();
        if let Some(address) = parse_bluetooth_address_from_device_id(&id) {
            log::info!(
                "[ble-windows] {context}: learned target address from paired device target={target_name:?} name={name:?} address={address:012X}"
            );
            return Some(address);
        }
    }
    None
}

/// One-line summary of an advertisement's manufacturer-data sections, for
/// diagnostic logging.
pub fn advertisement_manufacturer_data_summary(advertisement: &BluetoothLEAdvertisement) -> String {
    let Ok(manufacturer_data) = advertisement.ManufacturerData() else {
        return "mfg=unavailable".to_string();
    };
    let Ok(count) = manufacturer_data.Size() else {
        return "mfg=size-unavailable".to_string();
    };
    if count == 0 {
        return "mfg=none".to_string();
    }

    let mut entries = Vec::new();
    for index in 0..count {
        let Ok(entry) = manufacturer_data.GetAt(index) else {
            entries.push(format!("index={index}:unreadable"));
            continue;
        };
        let company_id = entry.CompanyId().unwrap_or_default();
        let bytes = entry
            .Data()
            .ok()
            .and_then(|buffer| buffer_to_vec(&buffer).ok())
            .unwrap_or_default();
        entries.push(format!(
            "company=0x{company_id:04X} data={}",
            hex_bytes(&bytes)
        ));
    }
    format!("mfg=[{}]", entries.join(";"))
}

/// Extract the Swift Pair display name embedded in an advertisement's
/// Microsoft (0x0006) manufacturer-data section, if present.
pub fn advertisement_swift_pair_display_name(
    advertisement: &BluetoothLEAdvertisement,
) -> Option<String> {
    let manufacturer_data = advertisement.ManufacturerData().ok()?;
    let count = manufacturer_data.Size().ok()?;
    for index in 0..count {
        let entry = manufacturer_data.GetAt(index).ok()?;
        let company_id = entry.CompanyId().ok()?;
        let bytes = entry
            .Data()
            .ok()
            .and_then(|buffer| buffer_to_vec(&buffer).ok())
            .unwrap_or_default();
        if let Some(name) = swift_pair_display_name_from_manufacturer_entry(company_id, &bytes) {
            return Some(name);
        }
    }
    None
}
