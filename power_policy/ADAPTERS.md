# Power policy v1 adapter contract

The shared core owns canonical blocker bits and deterministic active,
connected-idle, disconnected-idle, and shutdown-request decisions. Product
adapters own activity timestamps, settings persistence, charger/GPIO facts,
timers, scheduler calls, light-sleep/Stop entry, hardware shutdown, BLE
connection tuning, storage, diagnostics, LED, and display behavior.
