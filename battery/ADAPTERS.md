# Battery v1 adapter contract

The shared core owns voltage-to-percent conversion, charge-full debounce,
published-level normalization, notification decisions, and the standard
Battery Service (`0x180F`) / Battery Level (`0x2A19`) identifiers. Product
adapters own ADC acquisition and calibration, retained trim, charger GPIO,
timers, GATT registration/notification, storage, diagnostics, and UI effects.
