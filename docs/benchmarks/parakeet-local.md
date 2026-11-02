# Parakeet local benchmark status

No local Parakeet model was present in the Dicta Application Support cache when this integration was implemented. Models are never downloaded for tests or benchmarks without user consent, so there are no measured initialization, inference-duration, or real-time-factor results to report. Published FluidAudio benchmarks are hardware- and dataset-specific and are not presented as Dicta measurements.

To collect local numbers after explicitly installing a model, use a fixed local 16 kHz mono Float32 recording and record separately:

- model load/initialization wall time (cold and warm where possible);
- recording duration in seconds;
- inference wall time in seconds;
- real-time factor (`inference wall time / recording duration`);
- model/version, macOS version, Mac model/chip, and whether Core ML compilation cache was warm.

Do not upload recordings: Dicta audio is sensitive user data. This document intentionally contains no fabricated measurements.
