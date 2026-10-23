#!/usr/bin/env bash
# Package the release binary for one target into dist/.
#
# Shared by release.yml and release-build-check.yml so the packaging a pull
# request exercises is exactly what a release runs. Expects TARGET (a Rust
# target triple) and EXT (tar.gz or zip) in the environment, and the binary
# already built under target/$TARGET/release.
set -euo pipefail

: "${TARGET:?TARGET is required}"
: "${EXT:?EXT is required}"

# mise's github backend picks the asset by OS/arch in the name and strips a
# single top-level directory, so the archive holds one amb-<target>/ directory.
name="amb-${TARGET}"
bin=amb
if [ "${RUNNER_OS:-}" = "Windows" ]; then bin=amb.exe; fi

mkdir "$name"
cp "target/${TARGET}/release/${bin}" LICENSE README.md "$name/"
if [ "$EXT" = "zip" ]; then
  7z a "${name}.zip" "$name" > /dev/null
else
  tar czf "${name}.tar.gz" "$name"
fi

if command -v sha256sum > /dev/null; then
  sha256sum "${name}.${EXT}" > "${name}.${EXT}.sha256"
else
  shasum -a 256 "${name}.${EXT}" > "${name}.${EXT}.sha256"
fi

mkdir dist
mv "${name}.${EXT}" "${name}.${EXT}.sha256" dist/
