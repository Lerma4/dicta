#!/bin/sh
# Avoid Finder/AppleScript layout automation when creating a local unsigned DMG.
set -eu
if ! command -v cargo >/dev/null 2>&1; then . "$HOME/.cargo/env"; fi
npm run tauri -- build --bundles app
dicta_bundle=src-tauri/target/release/bundle
dicta_version=$(node -p "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json')).version")
dicta_arch=$(uname -m)
mkdir -p "$dicta_bundle/dmg/staging"
ditto "$dicta_bundle/macos/Dicta.app" "$dicta_bundle/dmg/staging/Dicta.app"
if [ ! -L "$dicta_bundle/dmg/staging/Applications" ]; then ln -s /Applications "$dicta_bundle/dmg/staging/Applications"; fi
hdiutil create -volname Dicta -srcfolder "$dicta_bundle/dmg/staging" -ov -format UDZO "$dicta_bundle/dmg/Dicta_${dicta_version}_${dicta_arch}.dmg"
