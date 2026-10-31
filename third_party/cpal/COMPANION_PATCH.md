# Companion WASAPI packet handling

This is the locally cached crates.io `cpal` 0.15.3 source, retained with its original licenses. The parent platform and Companion Type explicitly select this copy for the Windows capture candidate.

The scoped changes preserve Windows WASAPI input packet flags on `InputCallbackInfo` and replace a packet marked silent with aligned digital-silence samples before the callback reads it. A null pointer in a non-silent packet is reported as an error. ASIO reports no WASAPI flags. This enables the host to detect an unreliable timestamp or a discontinuity instead of silently treating it as a precise time.

The Windows loopback API is experimental until the two-track recording, durable save, device changes and real meeting have been verified. No live capture is performed by this dependency on its own.

The original crates.io archive SHA-256 is `873dab07c8f743075e57f524c583985fbaf745602acbe916a01539364369a779`.
