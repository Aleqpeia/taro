#!/usr/bin/env bash
# Deterministic screenshot driver for the Taro Bevy app.
#
# Taro is a graphical app with no testable "does it look right?" signal, so the
# way to drive it headlessly is the app's own TARO_CAPTURE harness: render one
# deterministic frame at a chosen wall-clock time, save a PNG, and exit. This
# wrapper sets BEVY_ASSET_ROOT (required when running the binary directly, or
# all card art 404s) and the capture env vars, then runs the binary.
#
# Usage:
#   capture.sh [options] OUTPUT.png
#
# Options:
#   -t SECONDS   capture time, wall-clock (TARO_CAPTURE_AT, default 5)
#   -s N         pre-select reading entry N, 0-9 (TARO_SELECT)
#   -r           start in reduced motion: instant, settled placement
#   -d SECONDS   fire one automatic redeal at this time (TARO_REDEAL_AT)
#   -T NAME      start in theme NAME: midnight|emerald|wine|ash (TARO_THEME)
#   -w WxH       initial window size, e.g. 1600x720 (TARO_WINDOW)
#   -c SECONDS   fire one theme cycle at this time (TARO_CYCLE_THEME_AT)
#   -q TEXT      seed the querent's question (TARO_QUESTION)
#   -F SECONDS   open the full-reading overlay at this time (TARO_SHOW_READING_AT)
#   -e           start with the question field focused (TARO_EDIT_QUESTION)
#   -R           use the release binary (target/release) instead of debug
#
# Run from the repo root. Reads the binary from target/{debug,release}/taro-app.
set -euo pipefail

at=5; select=""; reduced=""; redeal=""; theme=""; window=""; cycle=""
question=""; showread=""; edit=""; profile=debug
while getopts "t:s:rd:T:w:c:q:F:eR" opt; do
  case "$opt" in
    t) at=$OPTARG ;;
    s) select=$OPTARG ;;
    r) reduced=1 ;;
    d) redeal=$OPTARG ;;
    T) theme=$OPTARG ;;
    w) window=$OPTARG ;;
    c) cycle=$OPTARG ;;
    q) question=$OPTARG ;;
    F) showread=$OPTARG ;;
    e) edit=1 ;;
    R) profile=release ;;
    *) echo "see header for usage" >&2; exit 2 ;;
  esac
done
shift $((OPTIND - 1))

out=${1:-}
if [ -z "$out" ]; then echo "error: OUTPUT.png required" >&2; exit 2; fi

root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
bin="$root/target/$profile/taro-app"
if [ ! -x "$bin" ]; then
  echo "error: $bin not found — build first: cargo build -p taro-app${profile:+ --release}" >&2
  exit 1
fi

env_args=(BEVY_ASSET_ROOT="$root/crates/app" TARO_CAPTURE="$out" TARO_CAPTURE_AT="$at")
[ -n "$select" ]  && env_args+=(TARO_SELECT="$select")
[ -n "$reduced" ] && env_args+=(TARO_REDUCED_MOTION=1)
[ -n "$redeal" ]  && env_args+=(TARO_REDEAL_AT="$redeal")
[ -n "$theme" ]   && env_args+=(TARO_THEME="$theme")
[ -n "$window" ]  && env_args+=(TARO_WINDOW="$window")
[ -n "$cycle" ]   && env_args+=(TARO_CYCLE_THEME_AT="$cycle")
[ -n "$question" ] && env_args+=(TARO_QUESTION="$question")
[ -n "$showread" ] && env_args+=(TARO_SHOW_READING_AT="$showread")
[ -n "$edit" ]    && env_args+=(TARO_EDIT_QUESTION=1)

echo "running: ${env_args[*]} $bin" >&2
env "${env_args[@]}" "$bin" 2>&1 | grep -iE "capture|screenshot|panic|error" || true

if [ -f "$out" ]; then
  echo "ok: wrote $out" >&2
else
  echo "error: no screenshot produced" >&2
  exit 1
fi
