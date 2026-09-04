#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_WORKSPACE:?}" "${TAURI_RELEASE_TARGET:?}"

pnpm exec tauri "$@"
codesign --verify --deep --strict --verbose=2 \
  "${GITHUB_WORKSPACE}/target/${TAURI_RELEASE_TARGET}/release/bundle/macos/Vesper.app"
