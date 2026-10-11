# Architecture

`src/app` routes the main window; individual features consume the typed command adapter in `src/services/desktop.ts`. The independent `overlay.html` entry mounts only the overlay. Styles and shared controls support keyboard input, dark/light and reduced motion. No model, PCM or database is loaded into JavaScript. The dashboard chunk is lazy; its SVG graphs require no chart dependency.

Rust modules:

- `domain`: validated settings, sessions and aggregate DTOs.
- `state`: checked authoritative phase transitions, snapshot and recovery state.
- `application`: one dictation worker receiving start/stop/cancel controls. Capture lives on that worker; the CPAL callback keeps bounded mono PCM and sends levels at <=20 Hz. Inference stays off the GUI thread and keeps one warmed Whisper context. The worker blocks on controls while idle.
- `audio`: device selection, normalized PCM, RMS energy gate, windowed-sinc resampling with delay/tail compensation.
- `inference`: pinned catalog/download verification and persistent Whisper context. Cancellation/timeouts are local atomic state read by the native decoder.
- `platform`: public macOS AVFoundation/AX/AppKit bridge; all other platforms have explicit unavailable adapters. AX target tokens retain an exact editable element. No app titles/text are persisted.
- `persistence`: migration `user_version=1`, parameterized SQLite repositories/aggregates, UTC timestamps and local date/hour grouping, opt-in transcript table, retention and crash recovery.
- `commands`: typed narrow IPC, main-window guards for data/preferences/exports, native Save dialog with no arbitrary renderer-selected filesystem paths.

Initialization opens SQLite, recovers interrupted sessions, creates runtime/tray, registers configured hotkey and configures the persistent non-focusable overlay. Main close hides the window. Sleep/display-sleep cancels active capture/inference and clears physical hotkey state.

The stable adapter interfaces are functions and concrete resource handles rather than eight unused dynamic trait wrappers. OS behavior is isolated behind `platform`; replacing adapters does not require rewriting React or shared DTOs. The state-machine/SQLite/audio tests do not require a microphone or model; real inference is measured separately by the benchmark executable.

Recovery text is RAM-only unless transcript opt-in is active. Failed insertion does not imply failed transcription. The session remains completed with `insertion_recovery` classification and the snapshot marks `recovery_pending`. Recording cannot replace that pending result before explicit Copy. Ordinary crash recovery records an interrupted metric, but cannot recover volatile PCM or RAM-only transcripts after process termination.
