# Third-party notices for the native macOS app

FluidAudio and its Parakeet model weights are optional components. The app does not ship model weights; each model is downloaded only after the user presses **Install**. Model repository contents and license declarations can change, so verify the linked model card before redistributing downloaded assets.

## FluidAudio SDK 0.17.3

- Source: https://github.com/FluidInference/FluidAudio/tree/v0.17.3
- License: Apache License 2.0. The complete upstream license is bundled as `Licenses/FluidAudio-Apache-2.0.txt`.
- The package links the NemoTextProcessing XCFramework (text-processing-rs 0.3.1), as pinned in FluidAudio 0.17.3's `Package.swift`; its upstream license, NOTICE, third-party license list, and complete MIT license with rustfst/flate2 attributions are bundled under `Licenses/NemoTextProcessing-*`.

## Downloaded Parakeet models

The three exact Hugging Face repositories below currently declare Creative Commons Attribution 4.0 International (CC BY 4.0) in their model cards. The complete license text is bundled as `Licenses/Creative-Commons-Attribution-4.0.txt`. The app links each card in the Models screen; the model files are not included in the app bundle.

- Parakeet Ultra: https://huggingface.co/FluidInference/parakeet-ultra-coreml — model card declares CC BY 4.0; derived from `moondream/parakeet-ultra`.
- Parakeet V3: https://huggingface.co/FluidInference/parakeet-tdt-0.6b-v3-coreml — model card declares CC BY 4.0.
- Parakeet Redux: https://huggingface.co/FluidInference/parakeet-redux-coreml — model card declares CC BY 4.0; derived from `moondream/parakeet-redux`.

## Whisper

The native Whisper implementation uses the MIT-licensed whisper.cpp source in `Vendor/whisper.cpp`; see `Vendor/whisper.cpp/LICENSE`. Whisper model weights are downloaded separately from the repository and remain subject to their upstream terms.
