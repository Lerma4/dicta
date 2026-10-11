# Native audio, inference, and macOS adapters

## Audio capture

`audio::list_input_devices` enumerates CPAL input devices. `start_capture` opens only the selected/default microphone for the active recording, converts multichannel input to mono, stores a bounded PCM buffer, and publishes normalized per-buffer RMS/VAD events no more often than every 50 ms. `CaptureHandle::stop` resamples to 16 kHz with Rubato's windowed-sinc resampler before returning the samples; cancellation drops the stream and audio buffer. The orchestrator sets a five-minute limit, so captured audio is bounded to that duration.

The callback stores source-rate `f32` samples in Rust and never sends PCM through UI events. VAD is an RMS threshold per callback buffer; it detects audible energy and is not a learned speech classifier. The setting's threshold is clamped by validation to `0..=1`.

## Whisper and model files

The persistent `WhisperEngine` owns one `WhisperContext`; each transcription makes a new state while keeping loaded model weights warm. On Apple Silicon the Cargo target enables whisper-rs Metal; if GPU context initialization fails, loading retries with GPU disabled. Decoding uses a cooperative abort callback for cancellation and timeout. The callback is checked by whisper.cpp during decoding, so it cannot interrupt a native operation that does not return to the decoder.

`whisper-rs` 0.16's safe abort-closure trampoline casts its boxed trait object back to the concrete closure type. The adapter therefore uses its raw C callback API with a stack context whose lifetime covers the synchronous `full` call. Its callback only reads an `Arc<AtomicBool>` and a deadline. The same binding enables verbose C++ token logs in debug builds; the adapter installs a process-global no-op Whisper log callback before model loading so decoded words never reach stderr.

The catalog contains multilingual tiny, base, and small files from the official [ggerganov/whisper.cpp Hugging Face repository](https://huggingface.co/ggerganov/whisper.cpp), pinned to revision [`5359861c739e955e79d9a303bcbc70fb988958b1`](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1). Byte sizes and SHA-256 values come from the repository's file metadata at that revision:

| Model | Bytes | SHA-256 |
|---|---:|---|
| tiny | 77,691,713 | `be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21` |
| base | 147,951,465 | `60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe` |
| small | 487,601,967 | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` |

The user action that invokes `download_model` is the only code path that contacts Hugging Face. It checks the size and SHA-256 while streaming to a same-directory temporary file, reports progress at a bounded rate, honors cancellation, syncs the completed file, then renames it into place. A failed or canceled download removes the temporary file. `list_models` only checks file type and expected length to remain quick; `verified_model_path` hashes the selected file on the inference worker immediately before it is loaded.

`rubato` 0.16 is retained for its small fixed-input Sinc API and Rust 1.61 MSRV; Rubato 5 has a redesigned resampler API and would add a migration without a product benefit here. Upgrade when another maintained adapter requires the newer API or the pinned version stops receiving fixes. The app includes the upstream model repository's MIT metadata; verify the exact model and binding notices again when packaging a release. whisper-rs is Unlicense and whisper.cpp is MIT.

## macOS focus and text insertion

`capture_target` runs before the overlay is shown and retains the focused Accessibility element behind an opaque token. It rejects Dicta's own windows, non-editable or unidentified fields, secure fields, and missing Accessibility trust. Immediately before insertion, the native bridge verifies the same frontmost process and exact focused AX element, checks the secure-field role and selected-text writability again, then sets the public `kAXSelectedTextAttribute` so selected text is replaced or text is inserted at the caret. If any guard fails, insertion returns an error; the orchestrator keeps the result in RAM and shows the specific reason with an explicit Copy action. Starting from Dicta itself has no external target, so use the global shortcut in the app where the text belongs. It never writes to the clipboard automatically.

The explicit `copy_recovery_text` command writes to `NSPasteboard` only after the user invokes Copy. Accessibility status uses `AXIsProcessTrusted`; the request calls `AXIsProcessTrustedWithOptions` with Apple's prompt option. Microphone status and the first-use request use AVFoundation's public authorization API. The app bundle must include `NSMicrophoneUsageDescription`.

The public AppKit adapter positions within the visible frame of the display under the mouse, converted to top-left logical points relative to the primary display. `configure_overlay` applies transparency, `CanJoinAllSpaces` and `FullScreenAuxiliary` through the public `NSWindow` API; use it during setup with Tauri's native window handle. Tauri still owns the window's focusable/always-on-top settings. System monitoring exposes public workstation sleep and display sleep/wake events; macOS has no suitable public app API for reliably detecting every screen-lock event, so screen lock without display sleep is not covered.

## Dependencies and limits

The native core uses CPAL 0.18, Rubato 0.16, whisper-rs 0.16, reqwest 0.13 with Rustls, sha2 0.10, and the `cc` build dependency for the small macOS Objective-C bridge. The bridge links AppKit, AVFoundation, and ApplicationServices; all other operating systems compile explicit unavailable stubs. Build/test on the target macOS host is still required for microphone TCC, AX insertion, Spaces, fullscreen, multiple-display placement, and actual Metal acceleration. This source-level work does not claim those GUI/hardware smoke tests have run.
