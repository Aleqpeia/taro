#!/bin/sh
# Flatpak entry point. Bevy resolves assets from BEVY_ASSET_ROOT, so point it
# at the bundled directory that contains `assets/` (next to the binary).
export BEVY_ASSET_ROOT=/app/bin
exec /app/bin/taro-app "$@"
