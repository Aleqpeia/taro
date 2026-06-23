#!/usr/bin/env bash
# Build a Taro AppImage from a release build.
#
# Requirements:
#   * a release binary at target/release/taro-app (run `cargo build --release
#     -p taro-app` first, or pass --build to do it here);
#   * `appimagetool` on PATH, or its location in $APPIMAGETOOL.
#
# Output: dist/Taro-x86_64.AppImage
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

APPIMAGETOOL="${APPIMAGETOOL:-appimagetool}"
ARCH="${ARCH:-x86_64}"
BIN="target/release/taro-app"
APPDIR="$(mktemp -d)/Taro.AppDir"
OUT="dist/Taro-${ARCH}.AppImage"

if [[ "${1:-}" == "--build" ]]; then
    cargo build --release -p taro-app
fi
[[ -x "$BIN" ]] || { echo "error: $BIN not found; run a release build first" >&2; exit 1; }

echo "Assembling AppDir at $APPDIR"
mkdir -p "$APPDIR/usr/bin"
cp "$BIN" "$APPDIR/usr/bin/taro-app"
cp -r crates/app/assets "$APPDIR/usr/bin/assets"

install -m 0755 packaging/linux/AppRun "$APPDIR/AppRun"
cp packaging/linux/taro.desktop "$APPDIR/taro.desktop"
# Icon: Le Mat (The Fool, card 0) — apt for the project's first release.
cp crates/app/assets/cards/major_00.png "$APPDIR/taro.png"

mkdir -p dist
echo "Running appimagetool"
ARCH="$ARCH" "$APPIMAGETOOL" "$APPDIR" "$OUT"
echo "Built $OUT"
