# Device health v1 adapter contract

The shared core owns reset classification, saturating crash-window state,
startup-check aggregation, and runtime threshold decisions. Product adapters
map target reset reasons, retain the crash state, perform hardware checks,
sample metrics, schedule timers/tasks, emit observability events, and drive
LED or display effects.
