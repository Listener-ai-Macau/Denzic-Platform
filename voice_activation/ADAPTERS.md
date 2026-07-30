# Voice Activation Adapters

The platform owns speech-confirmation, silence-tail, duration, cooldown,
normalized transcript/phonetic relation, confirmation scheduling primitives,
post-activation PCM boundaries, and secondary-verifier fallback decisions.
Explicit secondary `Absent` evidence always blocks a later keyword-only
timeout fallback.

Product adapters own VAD/KWS/ASR inference, model and phrase configuration,
confirmation schedules and thresholds, PCM pre-roll storage, timers, recording
transport, power policy, LEDs, settings persistence, logging, and
physical-button overrides.
