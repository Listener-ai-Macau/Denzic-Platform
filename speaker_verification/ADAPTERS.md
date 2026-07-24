# Speaker Verification v1 Adapters

The platform owns only candidate gating policy. Product adapters own microphone
capture, model/runtime provisioning, speaker-embedding inference, protected
template storage, UI, ASR release, and rejected-audio disposal.

Automatic candidates fail closed when no enrolled template exists, inference is
unavailable, duration is outside the contract bounds, or the score does not meet
the product threshold. Manual recording bypasses the identity gate.
