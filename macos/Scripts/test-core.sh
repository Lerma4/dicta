#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
binary="$(mktemp -t dicta-core-tests)"
trap 'rm -f "$binary"' EXIT
swiftc -parse-as-library \
  Sources/DictaMacOS/DictationState.swift \
  Sources/DictaMacOS/AudioConverterInput.swift \
  Sources/DictaMacOS/TranscriptSafety.swift \
  Sources/DictaMacOS/TerminalRecovery.swift \
  Sources/DictaMacOS/RexPaste.swift \
  Sources/DictaMacOS/SessionStore.swift \
  Sources/DictaMacOS/LegacyReadOnlyRepository.swift \
  Sources/DictaMacOS/ModelCatalogService.swift \
  Tests/CoreScriptTests.swift \
  -lsqlite3 -o "$binary"
"$binary"
