#!/usr/bin/env bash
# Build and install the Taro flatpak (user installation).
#
# Requirements:
#   * a release binary at target/release/taro-app (or pass --build);
#   * flatpak with the flathub remote;
#   * flatpak-builder, or the org.flatpak.Builder flatpak (auto-installed).
#
# Result: `flatpak run io.github.aleqpeia.Taro`
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

APP_ID="io.github.aleqpeia.Taro"
MANIFEST="packaging/flatpak/${APP_ID}.yaml"
BUILD_DIR="target/flatpak-build"

if [[ "${1:-}" == "--build" ]]; then
    cargo build --release -p taro-app
fi
[[ -x target/release/taro-app ]] || {
    echo "error: target/release/taro-app not found; run a release build first (or pass --build)" >&2
    exit 1
}

# A user-level flathub remote (system remotes don't serve --user installs).
flatpak remote-add --user --if-not-exists flathub \
    https://dl.flathub.org/repo/flathub.flatpakrepo

if command -v flatpak-builder >/dev/null; then
    BUILDER=(flatpak-builder)
else
    flatpak info --user org.flatpak.Builder >/dev/null 2>&1 \
        || flatpak info org.flatpak.Builder >/dev/null 2>&1 \
        || flatpak install --user -y flathub org.flatpak.Builder
    BUILDER=(flatpak run org.flatpak.Builder)
fi

"${BUILDER[@]}" --user --install --install-deps-from=flathub --force-clean \
    "$BUILD_DIR" "$MANIFEST"

echo
echo "Installed. Run with: flatpak run $APP_ID"
