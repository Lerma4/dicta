# Dicta

Dicta is a native macOS app that transcribes speech locally and inserts the text into the active app. Audio and inference stay on your Mac. No account or internet connection is needed to dictate after installing a model. Transcript history is optional.

## Requirements

- macOS 13 or later
- Apple Silicon recommended for Parakeet
- Xcode Command Line Tools and CMake to build from source

## Speech models

The **Models** screen lists **Parakeet** first. Parakeet runs through FluidAudio and Core ML. Ultra and V3 require macOS 14 or later; Redux requires macOS 15. Parakeet Ultra is the default on compatible systems.

**WhisperFlow** is the alternative for less capable hardware or older macOS versions: Whisper models support macOS 13 and later. Models are not downloaded automatically. Choose **Install** in the Models screen. Inference never downloads models, and works offline once the selected model is installed.

## Build and install

Build the app from the repository root:

```sh
bash macos/Scripts/build-app.sh
```

The resulting bundle is `macos/.build/Dicta.app`. To install it in Applications:

```sh
ditto macos/.build/Dicta.app /Applications/Dicta.app
open /Applications/Dicta.app
```

The local build is ad-hoc signed and not notarized. On first launch, grant Microphone and Accessibility permissions when macOS requests them. Ad-hoc signing does not guarantee that permissions will remain valid after future rebuilds or app replacements.

## Dictation

With Dicta open, focus a text field in another app and press **⌘⇧Space** to start or stop dictation. Dicta inserts text only if the original app and verified accessibility target are still focused. If it cannot safely insert the text, recover the transcript in Dicta using the explicit **Copy** action.

Recording is limited to five minutes; Dicta is not a streaming transcription app. With Rex, Dicta temporarily uses the clipboard and **⌘V**, without pressing Return. It restores the previous clipboard contents only if no other app has changed them in the meantime.

## Data and privacy

Dicta stores dictation statistics in `~/Library/Application Support/app.dicta.native/sessions.sqlite3`. Transcript history is disabled by default; when enabled, transcripts are stored as plaintext on this Mac. Disabling history stops new transcripts from being saved but does not delete existing ones. The History screen can delete saved transcripts while retaining statistics. Audio is not saved.

Downloaded models are stored in the local Application Support cache. Deleting a model removes only that model's cache.

## Checks

```sh
sh macos/Scripts/test-core.sh
swift build -c release --package-path macos
```

`swift test --package-path macos` requires a toolchain that includes XCTest. Dependency licenses and attributions are listed in [`macos/THIRD_PARTY_NOTICES.md`](macos/THIRD_PARTY_NOTICES.md). The status of local Parakeet measurements is documented in [`docs/benchmarks/parakeet-local.md`](docs/benchmarks/parakeet-local.md).
