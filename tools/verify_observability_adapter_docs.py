#!/usr/bin/env python3
"""Keep the adapter-facing correlation and attribution contract explicit."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "observability" / "ADAPTERS.md"
REQUIRED = (
    "correlation_id",
    "event_sequence",
    "monotonic_ms",
    "edge_to_record_dispatch_ms",
    "ble_recovery_ms",
    "audio_first_packet_ms",
    "preview_latency_ms",
    "final_transcription_ms",
    "ota_transfer_ms",
    "error_category",
    "denzic_observability_v1_event_t",
    "EventEnvelope",
    "verify_adapter_compatibility.py",
)


def main():
    text = DOC.read_text(encoding="utf-8")
    missing = [item for item in REQUIRED if item not in text]
    if missing:
        raise SystemExit(f"observability adapter contract is missing: {', '.join(missing)}")


if __name__ == "__main__":
    main()
