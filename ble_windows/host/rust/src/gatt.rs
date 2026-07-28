//! Generic GATT characteristic I/O: buffered reads, writes with timeout, CCCD
//! (notify/indicate) enabling with a retry ladder, and tolerant optional
//! characteristic reads used for metadata-style characteristics.
//!
//! All UUIDs and cache policy come from the caller; nothing here knows about
//! any specific GATT service layout.

use std::time::Duration;

use windows::core::GUID;
use windows::Devices::Bluetooth::BluetoothCacheMode;
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus, GattDeviceService, GattWriteOption,
};
use windows::Foundation::IAsyncOperation;
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};

use crate::wait::{
    wait_async_operation_with_cancel, wait_gatt_communication_status, wait_gatt_write_result,
    BleCancel,
};
use crate::{DEFAULT_DISCOVERY_TIMEOUT_MS, OPTIONAL_READ_TIMEOUT_MS};

fn optional_read_timeout() -> Duration {
    Duration::from_millis(DEFAULT_DISCOVERY_TIMEOUT_MS)
        .min(Duration::from_millis(OPTIONAL_READ_TIMEOUT_MS))
}

pub fn buffer_to_vec(buffer: &IBuffer) -> windows::core::Result<Vec<u8>> {
    let length = buffer.Length()? as usize;
    let reader = DataReader::FromBuffer(buffer)?;
    let mut bytes = vec![0u8; length];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

pub fn bytes_to_buffer(bytes: &[u8]) -> Result<IBuffer, String> {
    let writer = DataWriter::new().map_err(|err| format!("BLE buffer writer failed: {err}"))?;
    writer
        .WriteBytes(bytes)
        .map_err(|err| format!("BLE buffer write failed: {err}"))?;
    writer
        .DetachBuffer()
        .map_err(|err| format!("BLE buffer detach failed: {err}"))
}

pub fn read_characteristic_bytes(
    characteristic: &GattCharacteristic,
    cache_mode: BluetoothCacheMode,
    label: &str,
    cancel: &BleCancel,
) -> Result<Vec<u8>, String> {
    read_characteristic_bytes_with_timeout(
        characteristic,
        cache_mode,
        label,
        Duration::from_millis(DEFAULT_DISCOVERY_TIMEOUT_MS),
        cancel,
    )
}

pub fn read_characteristic_bytes_with_timeout(
    characteristic: &GattCharacteristic,
    cache_mode: BluetoothCacheMode,
    label: &str,
    timeout: Duration,
    cancel: &BleCancel,
) -> Result<Vec<u8>, String> {
    let read = characteristic
        .ReadValueWithCacheModeAsync(cache_mode)
        .map_err(|err| format!("BLE {label} read failed: {err}"))?;
    let read = wait_async_operation_with_cancel(read, timeout, &format!("{label} read"), cancel)
        .map_err(|err| format!("BLE {label} read wait failed: {err}"))?;
    let status = read
        .Status()
        .map_err(|err| format!("BLE {label} read status failed: {err}"))?;
    if status != GattCommunicationStatus::Success {
        return Err(format!("BLE {label} read returned status={status:?}"));
    }
    buffer_to_vec(
        &read
            .Value()
            .map_err(|err| format!("BLE {label} read value failed: {err}"))?,
    )
    .map_err(|err| format!("BLE {label} read buffer failed: {err}"))
}

/// Start a GATT write and return the async operation without waiting.
/// Used for OTA WWR pipelining so the host can keep multiple ATT writes in flight.
pub fn start_gatt_write_with_option_async(
    characteristic: &GattCharacteristic,
    bytes: &[u8],
    write_option: GattWriteOption,
    label: &str,
) -> Result<IAsyncOperation<GattCommunicationStatus>, String> {
    let buffer = bytes_to_buffer(bytes)?;
    characteristic
        .WriteValueWithOptionAsync(&buffer, write_option)
        .map_err(|err| format!("BLE {label} write failed: {err}"))
}

pub fn write_gatt_value_with_timeout(
    characteristic: &GattCharacteristic,
    bytes: &[u8],
    write_option: GattWriteOption,
    timeout: Duration,
    label: &str,
) -> Result<GattCommunicationStatus, String> {
    let buffer = bytes_to_buffer(bytes)?;
    if write_option == GattWriteOption::WriteWithoutResponse {
        let operation = characteristic
            .WriteValueWithOptionAsync(&buffer, write_option)
            .map_err(|err| format!("BLE {label} write failed: {err}"))?;
        let status = wait_gatt_communication_status(operation, timeout, label)?;
        if status != GattCommunicationStatus::Success {
            return Err(format!("BLE {label} write returned status={status:?}"));
        }
        return Ok(status);
    }

    let operation = characteristic
        .WriteValueWithResultAndOptionAsync(&buffer, write_option)
        .map_err(|err| format!("BLE {label} write failed: {err}"))?;
    let result = wait_gatt_write_result(operation, timeout, label)?;
    let status = result
        .Status()
        .map_err(|err| format!("BLE {label} write status read failed: {err}"))?;
    let protocol_error = result
        .ProtocolError()
        .ok()
        .and_then(|value| value.Value().ok());
    if let Some(protocol_error) = protocol_error {
        log::warn!("[ble-windows] {label} write protocol_error={protocol_error}");
    }
    if status != GattCommunicationStatus::Success {
        let protocol_suffix = protocol_error
            .map(|value| format!(" protocol_error={value}"))
            .unwrap_or_default();
        return Err(format!(
            "BLE {label} write returned status={status:?}{protocol_suffix}"
        ));
    }
    Ok(status)
}

pub fn write_gatt_value_status_with_timeout(
    characteristic: &GattCharacteristic,
    bytes: &[u8],
    write_option: GattWriteOption,
    timeout: Duration,
    label: &str,
) -> Result<GattCommunicationStatus, String> {
    let buffer = bytes_to_buffer(bytes)?;
    let operation = characteristic
        .WriteValueWithOptionAsync(&buffer, write_option)
        .map_err(|err| format!("BLE {label} write failed: {err}"))?;
    let status = wait_gatt_communication_status(operation, timeout, label)?;
    if status != GattCommunicationStatus::Success {
        return Err(format!("BLE {label} write returned status={status:?}"));
    }
    Ok(status)
}

pub fn write_cccd_with_timeout(
    characteristic: &GattCharacteristic,
    value: GattClientCharacteristicConfigurationDescriptorValue,
    timeout: Duration,
) -> Result<GattCommunicationStatus, String> {
    let operation = characteristic
        .WriteClientCharacteristicConfigurationDescriptorWithResultAsync(value)
        .map_err(|err| format!("BLE CCCD write failed: {err}"))?;
    let result = wait_gatt_write_result(operation, timeout, "CCCD")?;
    let status = result
        .Status()
        .map_err(|err| format!("BLE CCCD write status read failed: {err}"))?;
    let protocol_error = result
        .ProtocolError()
        .ok()
        .and_then(|value| value.Value().ok());
    if let Some(protocol_error) = protocol_error {
        log::warn!("[ble-windows] CCCD write protocol_error={protocol_error}");
    }
    Ok(status)
}

/// Backoff for CCCD enable attempt `attempt` (1-based); attempts beyond the
/// ladder reuse the last delay.
pub fn cccd_enable_retry_delay(retry_delays: &[Duration], attempt: usize) -> Duration {
    retry_delays
        .get(attempt.saturating_sub(1))
        .copied()
        .unwrap_or_else(|| *retry_delays.last().expect("retry delays"))
}

fn cccd_value_label(value: GattClientCharacteristicConfigurationDescriptorValue) -> &'static str {
    if value == GattClientCharacteristicConfigurationDescriptorValue::Notify {
        "notify"
    } else if value == GattClientCharacteristicConfigurationDescriptorValue::Indicate {
        "indicate"
    } else {
        "configuration"
    }
}

/// Enable a CCCD (notify or indicate) with a retry ladder.
///
/// `operation_id` only labels log lines. `recovery_error` is consulted on each
/// failed attempt; when it returns `Some(message)` the ladder aborts early
/// with that message so the caller can switch to its own pairing recovery
/// instead of burning the retry budget.
pub fn write_cccd_with_retry(
    label: &str,
    operation_id: u64,
    characteristic: &GattCharacteristic,
    value: GattClientCharacteristicConfigurationDescriptorValue,
    timeout: Duration,
    retry_delays: &[Duration],
    recovery_error: Option<&dyn Fn(&str) -> Option<String>>,
) -> Result<GattCommunicationStatus, String> {
    let kind = cccd_value_label(value);
    let mut last_error: Option<String> = None;
    let mut last_status: Option<GattCommunicationStatus> = None;
    for attempt in 1..=retry_delays.len() + 1 {
        match write_cccd_with_timeout(characteristic, value, timeout) {
            Ok(GattCommunicationStatus::Success) => {
                return Ok(GattCommunicationStatus::Success);
            }
            Ok(status) => {
                if attempt > retry_delays.len() {
                    return Ok(status);
                }
                last_status = Some(status);
                let delay = cccd_enable_retry_delay(retry_delays, attempt);
                log::warn!(
                    "[ble-windows] {label} #{operation_id}: {kind} CCCD enable attempt {attempt} returned status={status:?}; retrying in {} ms",
                    delay.as_millis()
                );
                std::thread::sleep(delay);
            }
            Err(err) => {
                if let Some(recovery_error) = recovery_error.and_then(|probe| probe(&err)) {
                    log::warn!(
                        "[ble-windows] {label} #{operation_id}: {kind} CCCD enable attempt {attempt} hit recovery pairing window; entering pairing recovery instead of retrying CCCD: {err}"
                    );
                    return Err(recovery_error);
                }
                if attempt > retry_delays.len() {
                    return Err(err);
                }
                let delay = cccd_enable_retry_delay(retry_delays, attempt);
                log::warn!(
                    "[ble-windows] {label} #{operation_id}: {kind} CCCD enable attempt {attempt} failed: {err}; retrying in {} ms",
                    delay.as_millis()
                );
                last_error = Some(err);
                std::thread::sleep(delay);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        format!(
            "BLE CCCD {kind} write returned status={:?}",
            last_status.unwrap_or(GattCommunicationStatus::Unreachable)
        )
    }))
}

pub fn read_optional_string_characteristic(
    device: &BluetoothLEDevice,
    service_uuid: GUID,
    characteristic_uuid: GUID,
    cancel: &BleCancel,
) -> Option<String> {
    read_optional_characteristic_bytes(device, service_uuid, characteristic_uuid, cancel)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|value| value.trim_matches(char::from(0)).trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn read_optional_string_characteristic_from_service_with_timeout(
    service: &GattDeviceService,
    characteristic_uuid: GUID,
    cache_mode: BluetoothCacheMode,
    timeout: Duration,
    cancel: &BleCancel,
) -> Option<String> {
    read_optional_characteristic_from_service_with_timeout(
        service,
        characteristic_uuid,
        cache_mode,
        timeout,
        cancel,
    )
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .map(|value| value.trim_matches(char::from(0)).trim().to_string())
    .filter(|value| !value.is_empty())
}

pub fn read_optional_string_characteristic_from_service(
    service: &GattDeviceService,
    characteristic_uuid: GUID,
    cache_mode: BluetoothCacheMode,
    cancel: &BleCancel,
) -> Option<String> {
    read_optional_string_characteristic_from_service_with_timeout(
        service,
        characteristic_uuid,
        cache_mode,
        optional_read_timeout(),
        cancel,
    )
}

pub fn read_optional_u8_characteristic(
    device: &BluetoothLEDevice,
    service_uuid: GUID,
    characteristic_uuid: GUID,
    cancel: &BleCancel,
) -> Option<u8> {
    read_optional_characteristic_bytes(device, service_uuid, characteristic_uuid, cancel)
        .and_then(|bytes| bytes.first().copied())
}

pub fn read_optional_characteristic_bytes(
    device: &BluetoothLEDevice,
    service_uuid: GUID,
    characteristic_uuid: GUID,
    cancel: &BleCancel,
) -> Option<Vec<u8>> {
    let optional_timeout = optional_read_timeout();
    for cache_mode in [BluetoothCacheMode::Uncached] {
        let services_result = device
            .GetGattServicesForUuidWithCacheModeAsync(service_uuid, cache_mode)
            .ok()?;
        let services_result = wait_async_operation_with_cancel(
            services_result,
            optional_timeout,
            "optional GATT service discovery",
            cancel,
        )
        .ok()?;
        if services_result.Status().ok()? != GattCommunicationStatus::Success {
            continue;
        }
        let services = services_result.Services().ok()?;
        for index in 0..services.Size().ok()? {
            let service = services.GetAt(index).ok()?;
            let read_result = read_optional_characteristic_from_service(
                &service,
                characteristic_uuid,
                cache_mode,
                cancel,
            );
            let _ = service.Close();
            if read_result.is_some() {
                return read_result;
            }
        }
    }
    None
}

pub fn read_optional_characteristic_from_service_with_timeout(
    service: &GattDeviceService,
    characteristic_uuid: GUID,
    cache_mode: BluetoothCacheMode,
    timeout: Duration,
    cancel: &BleCancel,
) -> Option<Vec<u8>> {
    let timeout = timeout.min(Duration::from_millis(DEFAULT_DISCOVERY_TIMEOUT_MS));
    let operation = service
        .GetCharacteristicsForUuidWithCacheModeAsync(characteristic_uuid, cache_mode)
        .ok()?;
    let result = match wait_async_operation_with_cancel(
        operation,
        timeout,
        "optional characteristic",
        cancel,
    ) {
        Ok(result) => result,
        Err(err) => {
            log::debug!(
                "[ble-windows] optional characteristic {characteristic_uuid:?} discovery wait failed via {cache_mode:?}: {err}"
            );
            return None;
        }
    };
    let status = result.Status().ok()?;
    if status != GattCommunicationStatus::Success {
        log::debug!(
            "[ble-windows] optional characteristic {characteristic_uuid:?} discovery returned status={status:?} via {cache_mode:?}"
        );
        return None;
    }
    let characteristics = result.Characteristics().ok()?;
    if characteristics.Size().ok()? == 0 {
        log::debug!(
            "[ble-windows] optional characteristic {characteristic_uuid:?} not found via {cache_mode:?}"
        );
        return None;
    }
    let characteristic = characteristics.GetAt(0).ok()?;
    let operation = characteristic
        .ReadValueWithCacheModeAsync(cache_mode)
        .ok()?;
    let read = match wait_async_operation_with_cancel(
        operation,
        timeout,
        "optional characteristic read",
        cancel,
    ) {
        Ok(read) => read,
        Err(err) => {
            log::debug!(
                "[ble-windows] optional characteristic {characteristic_uuid:?} read wait failed via {cache_mode:?}: {err}"
            );
            return None;
        }
    };
    let status = read.Status().ok()?;
    if status != GattCommunicationStatus::Success {
        log::debug!(
            "[ble-windows] optional characteristic {characteristic_uuid:?} read returned status={status:?} via {cache_mode:?}"
        );
        return None;
    }
    buffer_to_vec(&read.Value().ok()?).ok()
}

pub fn read_optional_characteristic_from_service(
    service: &GattDeviceService,
    characteristic_uuid: GUID,
    cache_mode: BluetoothCacheMode,
    cancel: &BleCancel,
) -> Option<Vec<u8>> {
    read_optional_characteristic_from_service_with_timeout(
        service,
        characteristic_uuid,
        cache_mode,
        optional_read_timeout(),
        cancel,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cccd_retry_ladder_repeats_last_delay_beyond_ladder() {
        let delays = [
            Duration::from_millis(250),
            Duration::from_millis(750),
            Duration::from_millis(1500),
        ];
        assert_eq!(
            cccd_enable_retry_delay(&delays, 1),
            Duration::from_millis(250)
        );
        assert_eq!(
            cccd_enable_retry_delay(&delays, 2),
            Duration::from_millis(750)
        );
        assert_eq!(
            cccd_enable_retry_delay(&delays, 3),
            Duration::from_millis(1500)
        );
        assert_eq!(
            cccd_enable_retry_delay(&delays, 4),
            Duration::from_millis(1500)
        );
        assert_eq!(
            cccd_enable_retry_delay(&delays, 99),
            Duration::from_millis(1500)
        );
    }

    #[test]
    fn bytes_round_trip_through_winrt_buffer() {
        let payload = [0xDE, 0xAD, 0xBE, 0xEF];
        let buffer = bytes_to_buffer(&payload).expect("buffer writer should work");
        assert_eq!(
            buffer_to_vec(&buffer).expect("buffer reader should work"),
            payload
        );
    }

    #[test]
    fn optional_read_timeout_uses_generated_bounds() {
        assert_eq!(
            optional_read_timeout(),
            Duration::from_millis(OPTIONAL_READ_TIMEOUT_MS)
        );
    }
}
