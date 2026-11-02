# Bundled inference source

`whisper.cpp/` is the source tree shipped inside `whisper-rs-sys` 0.15.0 (whisper.cpp 1.8.3), copied from its Cargo registry source distribution. It is vendored so builds and the resulting app do not depend on a developer's Cargo registry path or a runtime Rust/CLI installation. The upstream whisper.cpp license is in `whisper.cpp/LICENSE`.

`Scripts/build-app.sh` builds a release static library for macOS 13+, then links it into the Swift executable. The source snapshot is part of this repository; updating it requires replacing the complete vendored tree and reviewing the upstream license/version.
