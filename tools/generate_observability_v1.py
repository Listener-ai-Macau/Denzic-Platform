#!/usr/bin/env python3
import argparse
import json
import re
import uuid
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "observability" / "protocol" / "observability_v1.json"
RUST_PATH = ROOT / "observability" / "host" / "rust" / "src" / "generated.rs"
C_PATH = ROOT / "observability" / "embedded" / "c" / "include" / "denzic_observability_v1_generated.h"

ENUMS = (
    ("event_sources", "EventSource", "event_source"),
    ("capabilities", "Capability", "capability"),
    ("ble_lifecycle_states", "BleLifecycleState", "ble_lifecycle_state"),
    ("command_results", "CommandResult", "command_result"),
    ("error_categories", "ErrorCategory", "error_category"),
    ("timing_metrics", "TimingMetric", "timing_metric"),
)

DIAG_LOG_GATT_UUID_KEYS = ("service_uuid", "control_uuid", "data_uuid", "count_uuid")
DIAG_LOG_GATT_SIZE_KEYS = ("event_wire_bytes", "chunk_header_bytes")
DIAG_LOG_GATT_CONTROL_OPS = ("start", "read", "stop")


def pascal(name):
    return "".join(part[:1].upper() + part[1:] for part in name.split("_"))


def c_symbol(name):
    return re.sub(r"[^A-Za-z0-9]+", "_", name).upper()


def validate_spec(spec):
    if spec.get("name") != "denzic_observability_v1" or spec.get("version") != 1:
        raise SystemExit("observability v1 name/version are fixed")
    for key, _, _ in ENUMS:
        values = spec.get(key)
        if not isinstance(values, list) or not values:
            raise SystemExit(f"{key} must be a non-empty array")
        codes = set()
        names = set()
        for item in values:
            name = item.get("name") if isinstance(item, dict) else None
            code = item.get("code") if isinstance(item, dict) else None
            if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*", name):
                raise SystemExit(f"{key} has invalid name: {name!r}")
            if not isinstance(code, int) or not 0 <= code <= 255:
                raise SystemExit(f"{key}.{name} must have a u8 code")
            if name in names or code in codes:
                raise SystemExit(f"{key} contains duplicate name or code")
            names.add(name)
            codes.add(code)
    if "unknown" not in {item["name"] for item in spec["ble_lifecycle_states"]}:
        raise SystemExit("ble_lifecycle_states must include unknown")
    if "none" not in {item["name"] for item in spec["error_categories"]}:
        raise SystemExit("error_categories must include none")
    if "none" not in {item["name"] for item in spec["timing_metrics"]}:
        raise SystemExit("timing_metrics must include none")

    gatt = spec.get("ble_diag_log_gatt")
    if not isinstance(gatt, dict):
        raise SystemExit("ble_diag_log_gatt must be an object")
    seen_uuids = set()
    for key in DIAG_LOG_GATT_UUID_KEYS:
        value = gatt.get(key)
        try:
            parsed = uuid.UUID(value if isinstance(value, str) else "")
        except (ValueError, AttributeError, TypeError) as error:
            raise SystemExit(
                f"ble_diag_log_gatt.{key} must be a canonical UUID: {error}"
            ) from error
        if str(parsed) != value or parsed in seen_uuids:
            raise SystemExit(f"ble_diag_log_gatt.{key} must be lowercase canonical and unique")
        seen_uuids.add(parsed)
    for key in DIAG_LOG_GATT_SIZE_KEYS:
        value = gatt.get(key)
        if not isinstance(value, int) or value <= 0:
            raise SystemExit(f"ble_diag_log_gatt.{key} must be a positive integer")
    if gatt.get("control_ops") != list(DIAG_LOG_GATT_CONTROL_OPS):
        raise SystemExit(f"ble_diag_log_gatt.control_ops must be {list(DIAG_LOG_GATT_CONTROL_OPS)}")
    for key in ("chunk_header_layout", "count_value", "pull_semantics"):
        value = gatt.get(key)
        if not isinstance(value, str) or not value:
            raise SystemExit(f"ble_diag_log_gatt.{key} must be a non-empty string")


def c_uuid_bytes(value):
    return ", ".join(f"0x{byte:02x}" for byte in reversed(uuid.UUID(value).bytes))


def render_rust_enum(items, rust_type):
    lines = [
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(u8)]",
        "#[serde(rename_all = \"snake_case\")]",
        f"pub enum {rust_type} {{",
    ]
    for item in items:
        lines.append(f"    {pascal(item['name'])} = {item['code']},")
    lines.append("}")
    return lines


def render_rust_gatt_constants(spec):
    gatt = spec["ble_diag_log_gatt"]
    lines = [
        "// BLE diagnostic log GATT service contract. The data characteristic",
        "// notifies one chunk per control read: a DIAG_LOG_CHUNK_HEADER_BYTES",
        "// header (event_count u16 LE, global_offset u32 LE, events_crc32 u32 LE,",
        "// CRC-32/IEEE over the payload) followed by event_count packed events of",
        "// DIAG_LOG_EVENT_WIRE_BYTES each.",
    ]
    for key in DIAG_LOG_GATT_UUID_KEYS:
        symbol = f"DIAG_LOG_GATT_{c_symbol(key)}"
        value = uuid.UUID(gatt[key])
        lines.append(f'pub const {symbol}: &str = "{value}";')
        lines.append(f"pub const {symbol}_U128: u128 = 0x{value.int:032x};")
    lines.append(f"pub const DIAG_LOG_EVENT_WIRE_BYTES: usize = {gatt['event_wire_bytes']};")
    lines.append(f"pub const DIAG_LOG_CHUNK_HEADER_BYTES: usize = {gatt['chunk_header_bytes']};")
    lines.append("")
    return lines


def render_rust(spec):
    lines = [
        "// Generated from observability/protocol/observability_v1.json. Do not edit.",
        "use serde::{Deserialize, Serialize};",
        "",
        f'pub const CONTRACT_NAME: &str = "{spec["name"]}";',
        f'pub const CONTRACT_VERSION: u8 = {spec["version"]};',
        "",
    ]
    for key, rust_type, _ in ENUMS:
        lines.extend(render_rust_enum(spec[key], rust_type))
        lines.append("")
    lines.extend(render_rust_gatt_constants(spec))
    lines.extend([
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "pub struct EventEnvelope {",
        "    pub contract_version: u8,",
        "    pub correlation_id: u64,",
        "    pub event_sequence: u32,",
        "    pub monotonic_ms: u32,",
        "    pub source: EventSource,",
        "    pub capability: Capability,",
        "    pub ble_lifecycle_state: BleLifecycleState,",
        "    pub command_result: CommandResult,",
        "    pub error_category: ErrorCategory,",
        "    pub timing_metric: TimingMetric,",
        "    pub timing_value_ms: u32,",
        "}",
        "",
        "impl EventEnvelope {",
        "    pub const fn new(",
        "        correlation_id: u64,",
        "        event_sequence: u32,",
        "        monotonic_ms: u32,",
        "        source: EventSource,",
        "        capability: Capability,",
        "    ) -> Self {",
        "        Self {",
        "            contract_version: CONTRACT_VERSION,",
        "            correlation_id,",
        "            event_sequence,",
        "            monotonic_ms,",
        "            source,",
        "            capability,",
        "            ble_lifecycle_state: BleLifecycleState::Unknown,",
        "            command_result: CommandResult::Started,",
        "            error_category: ErrorCategory::None,",
        "            timing_metric: TimingMetric::None,",
        "            timing_value_ms: 0,",
        "        }",
        "    }",
        "",
        "    pub const fn with_ble_lifecycle_state(mut self, value: BleLifecycleState) -> Self {",
        "        self.ble_lifecycle_state = value;",
        "        self",
        "    }",
        "",
        "    pub const fn with_result(mut self, value: CommandResult) -> Self {",
        "        self.command_result = value;",
        "        self",
        "    }",
        "",
        "    pub const fn with_error(mut self, value: ErrorCategory) -> Self {",
        "        self.error_category = value;",
        "        self",
        "    }",
        "",
        "    pub const fn with_timing(mut self, metric: TimingMetric, value_ms: u32) -> Self {",
        "        self.timing_metric = metric;",
        "        self.timing_value_ms = value_ms;",
        "        self",
        "    }",
        "}",
        "",
    ])
    return "\n".join(lines)


def render_c_enum(items, c_type, c_prefix):
    lines = ["typedef enum {"]
    for item in items:
        lines.append(f"    DENZIC_OBSERVABILITY_V1_{c_prefix}_{c_symbol(item['name'])} = {item['code']},")
    lines.append(f"}} denzic_observability_v1_{c_type}_t;")
    return lines


def render_c_gatt_constants(spec):
    gatt = spec["ble_diag_log_gatt"]
    lines = [
        "/* BLE diagnostic log GATT service contract. The data characteristic notifies",
        " * one chunk per control read: a chunk header (event_count u16 LE,",
        " * global_offset u32 LE, events_crc32 u32 LE, CRC-32/IEEE over the payload)",
        " * followed by event_count packed DIAG_LOG_EVENT_WIRE_BYTES events. */",
    ]
    for key in DIAG_LOG_GATT_UUID_KEYS:
        symbol = f"DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_{c_symbol(key)}"
        lines.append(f'#define {symbol}_TEXT "{gatt[key]}"')
        lines.append(f"#define {symbol}_BYTES {c_uuid_bytes(gatt[key])}")
    lines.append(
        f"#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES ({gatt['event_wire_bytes']}u)"
    )
    lines.append(
        f"#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES ({gatt['chunk_header_bytes']}u)"
    )
    lines.append("")
    return lines


def render_c(spec):
    lines = [
        "/* Generated from observability/protocol/observability_v1.json. Do not edit. */",
        "#ifndef DENZIC_OBSERVABILITY_V1_GENERATED_H",
        "#define DENZIC_OBSERVABILITY_V1_GENERATED_H",
        "",
        "#include <stddef.h>",
        "#include <stdint.h>",
        "",
        "#ifdef __cplusplus",
        "extern \"C\" {",
        "#endif",
        "",
        f'#define DENZIC_OBSERVABILITY_V1_CONTRACT_NAME "{spec["name"]}"',
        f'#define DENZIC_OBSERVABILITY_V1_CONTRACT_VERSION ({spec["version"]}u)',
        "",
    ]
    for key, _, c_type in ENUMS:
        lines.extend(render_c_enum(spec[key], c_type, c_symbol(c_type)))
        lines.append("")
    lines.extend(render_c_gatt_constants(spec))
    lines.extend([
        "typedef struct {",
        "    uint8_t contract_version;",
        "    uint64_t correlation_id;",
        "    uint32_t event_sequence;",
        "    uint32_t monotonic_ms;",
        "    denzic_observability_v1_event_source_t source;",
        "    denzic_observability_v1_capability_t capability;",
        "    denzic_observability_v1_ble_lifecycle_state_t ble_lifecycle_state;",
        "    denzic_observability_v1_command_result_t command_result;",
        "    denzic_observability_v1_error_category_t error_category;",
        "    denzic_observability_v1_timing_metric_t timing_metric;",
        "    uint32_t timing_value_ms;",
        "} denzic_observability_v1_event_t;",
        "",
        "static inline void denzic_observability_v1_event_init(",
        "    denzic_observability_v1_event_t *event,",
        "    uint64_t correlation_id,",
        "    uint32_t event_sequence,",
        "    uint32_t monotonic_ms,",
        "    denzic_observability_v1_event_source_t source,",
        "    denzic_observability_v1_capability_t capability)",
        "{",
        "    if (event == NULL) {",
        "        return;",
        "    }",
        "    event->contract_version = DENZIC_OBSERVABILITY_V1_CONTRACT_VERSION;",
        "    event->correlation_id = correlation_id;",
        "    event->event_sequence = event_sequence;",
        "    event->monotonic_ms = monotonic_ms;",
        "    event->source = source;",
        "    event->capability = capability;",
        "    event->ble_lifecycle_state = DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_UNKNOWN;",
        "    event->command_result = DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_STARTED;",
        "    event->error_category = DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_NONE;",
        "    event->timing_metric = DENZIC_OBSERVABILITY_V1_TIMING_METRIC_NONE;",
        "    event->timing_value_ms = 0u;",
        "}",
        "",
        "#ifdef __cplusplus",
        "}",
        "#endif",
        "",
        "#endif",
        "",
    ])
    return "\n".join(lines)


def write_or_check(path, expected, check):
    if check:
        actual = path.read_text(encoding="utf-8") if path.exists() else None
        if actual != expected:
            raise SystemExit(f"generated file is stale: {path.relative_to(ROOT)}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(expected, encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    spec = json.loads(SPEC_PATH.read_text(encoding="utf-8"))
    validate_spec(spec)
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
