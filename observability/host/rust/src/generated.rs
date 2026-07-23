// Generated from observability/protocol/observability_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_observability_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    Firmware = 1,
    Type = 2,
    Transport = 3,
    Provider = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Ble = 1,
    Audio = 2,
    Ota = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum BleLifecycleState {
    Unknown = 0,
    Disconnected = 1,
    Advertising = 2,
    Connecting = 3,
    ConnectedIdle = 4,
    Recording = 5,
    Recovering = 6,
    PairedElsewhere = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum CommandResult {
    Started = 1,
    Succeeded = 2,
    Cancelled = 3,
    Failed = 4,
    Timeout = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    None = 0,
    Device = 1,
    Transport = 2,
    Host = 3,
    Provider = 4,
    Network = 5,
    Protocol = 6,
    Resource = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum TimingMetric {
    None = 0,
    EdgeToRecordDispatchMs = 1,
    BleRecoveryMs = 2,
    AudioFirstPacketMs = 3,
    PreviewLatencyMs = 4,
    FinalTranscriptionMs = 5,
    OtaTransferMs = 6,
}

// BLE diagnostic log GATT service contract. The data characteristic
// notifies one chunk per control read: a DIAG_LOG_CHUNK_HEADER_BYTES
// header (event_count u16 LE, global_offset u32 LE, events_crc32 u32 LE,
// CRC-32/IEEE over the payload) followed by event_count packed events of
// DIAG_LOG_EVENT_WIRE_BYTES each.
pub const DIAG_LOG_GATT_SERVICE_UUID: &str = "710af845-6d9f-6583-0c4d-9e5b3bc3093a";
pub const DIAG_LOG_GATT_SERVICE_UUID_U128: u128 = 0x710af8456d9f65830c4d9e5b3bc3093a;
pub const DIAG_LOG_GATT_CONTROL_UUID: &str = "710af845-6d9f-6583-0c4d-9e5b3bc3093b";
pub const DIAG_LOG_GATT_CONTROL_UUID_U128: u128 = 0x710af8456d9f65830c4d9e5b3bc3093b;
pub const DIAG_LOG_GATT_DATA_UUID: &str = "710af845-6d9f-6583-0c4d-9e5b3bc3093c";
pub const DIAG_LOG_GATT_DATA_UUID_U128: u128 = 0x710af8456d9f65830c4d9e5b3bc3093c;
pub const DIAG_LOG_GATT_COUNT_UUID: &str = "710af845-6d9f-6583-0c4d-9e5b3bc3093d";
pub const DIAG_LOG_GATT_COUNT_UUID_U128: u128 = 0x710af8456d9f65830c4d9e5b3bc3093d;
pub const DIAG_LOG_EVENT_WIRE_BYTES: usize = 24;
pub const DIAG_LOG_CHUNK_HEADER_BYTES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub contract_version: u8,
    pub correlation_id: u64,
    pub event_sequence: u32,
    pub monotonic_ms: u32,
    pub source: EventSource,
    pub capability: Capability,
    pub ble_lifecycle_state: BleLifecycleState,
    pub command_result: CommandResult,
    pub error_category: ErrorCategory,
    pub timing_metric: TimingMetric,
    pub timing_value_ms: u32,
}

impl EventEnvelope {
    pub const fn new(
        correlation_id: u64,
        event_sequence: u32,
        monotonic_ms: u32,
        source: EventSource,
        capability: Capability,
    ) -> Self {
        Self {
            contract_version: CONTRACT_VERSION,
            correlation_id,
            event_sequence,
            monotonic_ms,
            source,
            capability,
            ble_lifecycle_state: BleLifecycleState::Unknown,
            command_result: CommandResult::Started,
            error_category: ErrorCategory::None,
            timing_metric: TimingMetric::None,
            timing_value_ms: 0,
        }
    }

    pub const fn with_ble_lifecycle_state(mut self, value: BleLifecycleState) -> Self {
        self.ble_lifecycle_state = value;
        self
    }

    pub const fn with_result(mut self, value: CommandResult) -> Self {
        self.command_result = value;
        self
    }

    pub const fn with_error(mut self, value: ErrorCategory) -> Self {
        self.error_category = value;
        self
    }

    pub const fn with_timing(mut self, metric: TimingMetric, value_ms: u32) -> Self {
        self.timing_metric = metric;
        self.timing_value_ms = value_ms;
        self
    }
}
