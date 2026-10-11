# Dicta — handoff / progress

Updated 2026-10-11, around 22:36 (Europe/Rome). Save point before final validation; do not assume all acceptance checks are complete.

User: build the application described in SPECIFICATION.md, name Dicta; orchestrate with Sol, delegate to GPT-6 Luna; initialize local Git and make organized commits; do not push. Avoid non-blocking questions. If context/token budget becomes low, update this file before stopping.

Repository is deliberately nested and independent from parent 360drone-shopify repository. Initial requirements commit made on main. No remote configured.

Environment: macOS 26.6, arm64, Xcode CommandLineTools; Node 24.21.0 / npm 11.19.0; Rust stable 1.99.0 installed via official rustup, executable ~/.cargo/bin/cargo. CMake already installed.

Agents work in shared checkout: ui owns React/config/package; persistence owns domain and SQLite; native owns audio/inference/platform. Parent owns Rust shell/runtime/commands, CI/scripts and integration. Shared schemas/APIs: CONTRACT.md.

Implemented: Italian React UI/main + independent overlay; real CPAL mono capture and bounded RMS events; sinc16kHz resampling; persistent Whisper context/Metal CPU fallback; pinned verified/cancellable downloads tiny/base/small; public macOS AX guarded insertion, permissions, display/Spaces, sleep events, tray/global shortcuts/autostart; SQLite migrations/local statistics/history opt-in/export/reset/crash recovery; state machine; CI config and docs.

Important findings fixed: resampler truncated 43 tail samples at 48kHz; upstream whisper-rs safe abort callback has mismatched userdata representation causing immediate encode abort/UB (use current raw lifetime-bound callback); Whisper debug logging can leak decoded tokens (global no-op logger installed); recovery pending result cannot be replaced before Copy; explicit native clipboard for history; full reset includes models/preferences; native Save dialog avoids renderer arbitrary paths.

Checks at this save point: frontend 14 RTL/Vitest tests and prior lint/typecheck/build pass. Rust 14 tests and lib clippy pass per agent, final parent all-targets checks in flight after export/reset edits. Parent frontend final run hit no-undef `process` in new benchmark CSV script; fixed with explicit node:process import, must rerun. Two real release benchmarks succeeded on M4 (JFK and Italian system-TTS) with verified Tiny; JSON/CSV baselines in docs/benchmarks and PERFORMANCE.md. Model/sample test assets only in /tmp/dicta-benchmark (not repo/app data).

Bundle attempt produced Dicta.app but Tauri's Finder DMG layout script failed after producing a temporary writable image. Do not call DMG complete. Need finish final app rebuild after log-sink/export changes; prefer app target and headless hdiutil fallback for DMG. Native .app GUI startup still needs observation, without granting OS microphone/AX permissions automatically.

Commits: f1e7e3b requirements/contract, 4697bea UI. Backend + docs remain uncommitted at this save point; make coherent core and validation/docs commits after checks. No remote/push/tag/release. Git root is this nested Dicta directory.

Account limit latest observation: 91% short-window used (9% remaining), 14% weekly used. Tool reports rate percentages, not an exact per-chat token count; don't invent token totals. Agents all delivered and should not modify files further. Parent continues integration.

Final-validation update: parent lint, 14 frontend tests, typecheck/build, Rust 14 tests, all-targets Clippy -D warnings, cargo fmt check, production npm audit (0 vulnerabilities), benchmark CSV self-check all pass. Final .app release build succeeds (20.76 MiB). Native GUI launched through computer-use: main onboarding receives actual Settings/Permissions/catalog and correctly shows 0/3/no model; dashboard command receives main-window guard and query loading is being observed. No TCC permissions granted, no user mic recorded, no live AX insertion tested. Core committed109a99c; integration frontend fixes0644b42. Headless DMG script currently running.

Next: final checks, native app smoke + empty dashboard/settings/model views, finish app/DMG packaging without OS permission changes, coherent commits, clean Git status, update this file with exact observed results and limitations. Personal mic/AX permissions and live insertion/Spaces/fullscreen remain manual. P1/P2 and Windows/Linux native adapters are explicitly deferred; no production/signing claims. Preserve privacy/no fake dashboard metrics.

## Final handoff

The local implementation and available checks are complete for this pass. Parent observed the bundled desktop onboarding, successfully queried the genuinely empty SQLite dashboard (no fake session data), clicked explicit Download in Dicta, and observed Whisper Tiny “Installato” / setup 1 of 3. SHA-256 acceptance occurred through the production downloader. Model is now installed in `~/Library/Application Support/app.dicta.desktop/models/ggml-tiny.bin`; microphone and AX trust remain ungranted. The app is running and awaiting these user-controlled permissions. Screenshot: `docs/dicta-desktop.png`.

Final .app: `src-tauri/target/release/bundle/macos/Dicta.app` (20.76 MiB). DMG: `src-tauri/target/release/bundle/dmg/Dicta_0.1.0_arm64.dmg`, successfully generated by `scripts/package-macos.sh` using headless hdiutil after standard Tauri Finder layout failed. Both are unsigned local alpha artifacts. No hidden CLI/runtime needed on the end-user Mac. Build outputs/model/audio aren't in Git.

Verified: frontend lint/14 tests/typecheck/build; Rust format/14 tests/all-targets clippy -D warnings; release .app build; production npm audit zero vulnerabilities; benchmark CSV self-check; shell syntax checks. Actual English + Italian Tiny inference benchmarks and JSON/CSV baseline are in PERFORMANCE.md. General mic quality/WER and full shortcut→mic→overlay→AX field flow are not yet measured. There are no mock production metrics.

Remaining acceptance work: grant TCC yourself in app/System Settings, run harmless TextEdit mic/insertion/focus tests; Spaces/fullscreen/multi-monitor/Dock, revoke/regrant, crash/reboot/autostart/sleep tests; latency and idle measurements; signing/notarization and exhaustive transitive license packaging before distribution. Screen-lock-only events are an explicitly documented limitation. Runtime recovery is volatile without history opt-in; no audio archive exists. UI/feature shortcomings such as dedicated microphone calibration onboarding and richer retention/date-range history deletion remain future refinements, not completed claims.

Git: independent nested repository on main, no remotes/push/tags/releases. Four implementation commits already present (`f1e7e3b`, `4697bea`, `109a99c`, `0644b42`); final docs/validation commit follows this file. See `git log --oneline` for final identity. CI has been prepared but cannot have run remotely without a push. Do not push until the user supplies the destination.
