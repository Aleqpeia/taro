---
name: run-taro
description: Build, run, and screenshot the Taro Bevy desktop app (animated Tarot de Marseille Celtic Cross). Use to launch Taro, capture a deterministic frame, verify visuals, or confirm a UI/render change works.
---

# Run Taro

Taro is a Rust/Bevy 0.18 desktop app that deals an animated Celtic Cross Tarot
spread. It has no logs-based "does it look right?" signal, so you drive it
through its built-in **capture harness**: render one deterministic frame at a
chosen wall-clock time, save a PNG, exit — then Read the PNG back (the Read tool
renders images = your eyes). Tweens are keyed to elapsed time, so the capture
time picks a precise animation frame.

The driver is `.claude/skills/run-taro/capture.sh`. All paths below are relative
to the repo root; run commands from there.

## Prerequisites

A Vulkan driver — works on the Mesa software rasterizer (`llvmpipe`), no GPU
needed. This container already has it (`libvulkan_lvp.so`) plus a Wayland/X
display, so **no `xvfb` is required**. On a bare box, the runtime libs are:

```bash
# Fedora
sudo dnf install vulkan-loader mesa-vulkan-drivers libxkbcommon wayland libX11 libXcursor libXi libXrandr
```

Bevy's feature set is deliberately curated (`default-features=false`, no
`audio`/`gamepad`) so ALSA and libudev are **not** needed. Don't add the
`2d`/`ui`/`audio`/`default_platform` umbrella features — they'll fail to link.

## Build

```bash
cargo build -p taro-app          # debug: watchable, fine for captures
cargo build -p taro-app --release  # smoother on llvmpipe; pass -R to the driver
```

Domain logic (deck, spreads, meanings) is a separate engine-agnostic crate:

```bash
cargo test -p taro-domain        # 8 tests, ~instant
```

## Run (agent path) — the driver

```bash
# Settled spread, default selection, captured at 5s
.claude/skills/run-taro/capture.sh out.png

# Reduced motion (instant placement, settled by ~1.5s) + pre-select entry 3
.claude/skills/run-taro/capture.sh -r -s 3 -t 1.5 out.png

# A theme at a non-default window size (verify a re-skin or resize scaling)
.claude/skills/run-taro/capture.sh -r -T emerald -w 1600x720 -t 1.5 out.png

# Seed a question and open the woven full-reading overlay (Phase 4)
.claude/skills/run-taro/capture.sh -r -q "Will the move work out?" -F 1.0 -t 1.6 out.png
```

Then Read `out.png`. Options:

| Flag | Effect |
| --- | --- |
| `-t SECONDS` | capture time, wall-clock (default 5; use ~1.5 with `-r`) |
| `-s N` | pre-select reading entry N (0–9); panel + gold halo follow it |
| `-r` | reduced motion — instant, fully-settled placement |
| `-d SECONDS` | fire one automatic redeal at this time (exercise the deal animation) |
| `-T NAME` | start in a theme: `midnight` (default), `emerald`, `wine`, `ash` |
| `-w WxH` | initial window size, e.g. `1600x720` — exercises fit-to-window scaling |
| `-c SECONDS` | fire one runtime theme cycle at this time (exercises the in-place re-skin) |
| `-q TEXT` | seed the querent's question (shown in the top banner; frames the full reading) |
| `-F SECONDS` | open the full-reading overlay at this time (woven prose across all 10 cards) |
| `-e` | start with the question field focused — screenshots the blinking caret |
| `-R` | use `target/release/taro-app` instead of debug |

The driver sets `BEVY_ASSET_ROOT` for you (required when running the binary
directly, or all card art 404s) and filters the run to capture/error lines.

**Close-ups:** the Read tool downscales the 1280×860 window. To inspect a card
or the reading-panel text, crop first (stdlib-only, no PIL/numpy):

```bash
# pngcrop.py in.png out.png x y w h [scale] — the reading panel is on the right
python3 .claude/skills/run-taro/pngcrop.py out.png panel.png 830 40 420 400 2
```

## Run (human path)

```bash
cargo run --release -p taro-app   # opens a window; click cards, Space = redeal, R = reduced motion
```

Useless headless (it just waits on a window) — for an agent, always use the
driver. Ctrl-C to quit.

## Gotchas

- **`BEVY_ASSET_ROOT` is mandatory** when launching the binary directly. Without
  it Bevy resolves assets against `target/debug/assets` and every card texture
  404s (blank cards). The driver sets it; only matters if you run the binary by
  hand.
- **Capture is async with a grace period.** The screenshot is handed to the
  render world over a channel; `capture.rs` waits ~1.5s after requesting it
  before sending `AppExit`. So total run ≈ `TARO_CAPTURE_AT + 1.5s`. Don't kill
  the process early or you get a truncated/missing PNG.
- **Pick capture time to match motion.** Without `-r`, the deal+flip animation is
  still in flight before ~5s — capturing at 2–3s deliberately catches a mid-deal
  frame (cards face-down, fanning out). For the settled spread use `-t 5` or add
  `-r` and `-t 1.5`.
- **The panel never spoils a card before its flip** — if you capture mid-deal and
  the selected card hasn't flipped, the panel shows placeholder text by design,
  not a bug.
- **llvmpipe is slow.** A debug capture takes ~5s of wall clock beyond the
  capture time; that's the software rasterizer, not a hang.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Blank/white cards in the shot | `BEVY_ASSET_ROOT` not set — use the driver, or export `BEVY_ASSET_ROOT="$PWD/crates/app"`. |
| `taro-app not found` | `cargo build -p taro-app` first (add `--release` and pass `-R` for the release path). |
| "no screenshot produced" | The run panicked or was killed before the grace period — re-run and read the captured error lines. |
| Linker errors mentioning `asound`/`udev` | Someone added an umbrella Bevy feature; keep the curated set in `crates/app/Cargo.toml`. |
