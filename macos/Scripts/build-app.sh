#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
cmake -S Vendor/whisper.cpp -B .build/whisper \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=13.0 \
  -DBUILD_SHARED_LIBS=OFF \
  -DGGML_NATIVE=OFF \
  -DGGML_CCACHE=OFF \
  -DCMAKE_ARCHIVE_OUTPUT_DIRECTORY="$PWD/.build/whisper/lib" \
  -DWHISPER_BUILD_TESTS=OFF \
  -DWHISPER_BUILD_EXAMPLES=OFF \
  -DWHISPER_BUILD_SERVER=OFF \
  -DGGML_METAL=OFF \
  -DGGML_BLAS=OFF \
  -DGGML_ACCELERATE=ON \
  -DGGML_OPENMP=OFF
cmake --build .build/whisper --config Release --parallel
swift build -c release
app="$PWD/.build/Dicta.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp .build/release/DictaMacOS "$app/Contents/MacOS/DictaMacOS"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>DictaMacOS</string>
<key>CFBundleIdentifier</key><string>app.dicta.native</string>
<key>CFBundleName</key><string>Dicta</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>1.0</string>
<key>LSMinimumSystemVersion</key><string>13.0</string>
<key>NSMicrophoneUsageDescription</key><string>Dicta usa il microfono per registrare la dettatura richiesta.</string>
</dict></plist>
PLIST
codesign --force --deep --sign - "$app"
printf 'Built %s\nLaunch with: open "%s"\n' "$app" "$app"
