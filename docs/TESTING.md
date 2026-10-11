# Validation

Observed on 2026-10-11: parent lint, 14 frontend tests, typecheck/build, 14 Rust tests, all-targets Clippy with denied warnings, cargo fmt check, production npm audit (0 vulnerabilities), benchmark CSV self-check, and final Tauri .app release build passed. The bundled app launched and its onboarding rendered actual permission/settings/model values. No OS microphone/Accessibilità permissions were granted during this validation. Live capture/AX/Spaces checks below remain manual.

Native smoke also verified the empty SQLite dashboard and a successful explicit in-app Tiny model download: UI showed “Installato” and setup 1/3. A local unsigned DMG was generated successfully with the headless packaging script. Screenshot is `dicta-desktop.png`. These checks do not prove actual microphone capture or text insertion.

Run the commands in README. Unit tests need neither microphone nor downloaded model/network. Frontend coverage includes browser gating, keyboard routes, real-empty dashboard, query errors/filter changes, overlay states/actual levels/recovery, failed settings saves and retention/preferences controls. Rust coverage includes phase transition/idempotence guards, PCM resampling duration/tail and anti-alias behavior, energy thresholds, catalog validation, SQLite opt-in/reset/retention/export/search/percentiles/UTC-local buckets and file-backed WAL purge.

Inference smoke/benchmark is separate and uses the production Whisper adapter and an explicitly downloaded model. See PERFORMANCE.md. No user speech is uploaded or used in a test. Browser view verification exercises layout only; Tauri IPC must be validated in the native app. The first compile/test cycle corrected CPAL API differences and a resampler tail-loss defect. Real model inference found the upstream callback issue described in ADR-001.

Manual checklist — mark only after actual observations:

- [ ] Microfono first permission, denial and revoke/regrant with bundled app.
- [ ] Accessibilità permission and revoke/regrant; never inspect passwords.
- [ ] Global hotkey in a harmless TextEdit field, hold/toggle, repeated events and release outside focus.
- [ ] Speak Italian, stop, verify inserted text only once at the original caret/selection.
- [ ] Change app or field during recording; observe recovery instead of insertion.
- [ ] Click overlay cancel; retain main app focus and stop microphone promptly.
- [ ] Empty/silent input, disconnected/occupied mic, device switch, model failure and memory pressure.
- [x] Explicit in-app Tiny download / checksum acceptance / installed state.
- [ ] Download cancel/retry and truncated/bad-checksum file rejection via GUI (catalog/parser guards have unit coverage).
- [ ] Second monitor, Spaces, fullscreen, Dock position and reduced motion.
- [ ] Sleep/wake; lock without display sleep remains the documented gap.
- [ ] Opt-in history, search/copy/delete/export; disable history and verify deletion.
- [ ] Native CSV/JSON Save dialog, full reset, autostart, tray-only use and restart.
- [ ] Process crash/restart and Mac reboot; interrupted metrics are recovered, volatile audio/text is not.

CI file validates frontend, format, clippy/tests and native release builds on macOS/Windows/Linux. It does not change branch protection or mandatory merge rules. Since no remote/push has been configured, remote CI has not executed. Signing/notarization and personal hardware/TCC workflows cannot be replaced by passing unit tests.
