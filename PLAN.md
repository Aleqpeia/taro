# Taro — Animated Tarot de Marseille Fortune Telling

A Linux desktop app (Rust) that performs animated tarot readings using the
Tarot de Marseille, starting with the Celtic Cross spread.

## Decisions (locked)

| Area           | Choice                                                                |
| -------------- | --------------------------------------------------------------------- |
| Language       | Rust (2021 edition)                                                   |
| Rendering / UI | **Bevy** (2D ECS game engine) + `bevy_tweening` for card animation    |
| Interpretation | **Hybrid** — authored static meanings (offline) + optional Claude API |
| Card art       | **Public-domain TdM scans** (e.g. Jean Dodal 1701 / Conver 1760), 78  |
| Target         | Linux desktop; package as AppImage and Flatpak                        |

## Why Bevy

Card dealing, shuffling, flipping, glow on the significator, easing arcs — these
are sprite-animation problems, and a 2D game engine is the natural fit. Bevy
gives us: ECS for managing 78 cards + 10 spread positions cleanly, a built-in
`States` machine for the reading flow, GPU sprite batching at 60 fps, and
`bevy_tweening` for declarative position/rotation/scale/color tweens. The cost is
a heavier dependency and a game-loop programming model (systems over callbacks),
which is acceptable here.

---

## Architecture

Core domain logic is **engine-agnostic** (plain Rust, unit-testable, no Bevy
types). Bevy sits on top as the presentation + animation layer. This keeps the
tarot rules testable and lets us swap or add a UI later.

```
crates/ (or modules in one binary crate to start)
├── domain/                 # pure logic, no Bevy
│   ├── card.rs             # Card, Arcana, Suit, Rank, Orientation
│   ├── deck.rs             # full 78-card deck construction
│   ├── shuffle.rs          # seeded shuffle + draw (rand)
│   ├── spread.rs           # Spread trait + Position metadata
│   ├── spreads/
│   │   └── celtic_cross.rs # 10 positions, layout offsets, position meanings
│   └── meanings.rs         # load + look up authored interpretations
├── data/                   # bundled, not code
│   ├── meanings.ron        # per-card upright/reversed + per-position text
│   └── spreads.ron         # position definitions (optional, or in code)
├── assets/
│   └── cards/              # 78 PNGs + card_back.png  (Bevy asset dir)
└── app/  (Bevy)
    ├── main.rs            # App setup, plugins, asset loading
    ├── states.rs         # AppState enum
    ├── layout.rs         # maps Position -> world Transform
    ├── animation.rs      # shuffle / deal / flip systems (bevy_tweening)
    ├── reading_ui.rs     # panels: position label, meaning text
    └── ai.rs             # optional Claude "deeper reading" (async)
```

### Domain model

```rust
enum Arcana { Major, Minor }
enum Suit { Cups, Coins, Swords, Batons }      // Coupes, Deniers, Épées, Bâtons
enum Rank { Ace..Ten, Valet, Cavalier, Reine, Roi } // pip + 4 courts
enum Orientation { Upright, Reversed }

struct Card { id: CardId, arcana: Arcana, /* major no., or suit+rank */ }
struct DrawnCard { card: Card, orientation: Orientation }
```

`CardId` is a stable key (e.g. `le_mat`, `cups_03`) used to index both the art
file and the meanings table.

### Spread abstraction (extensible from day one)

```rust
struct PositionDef {
    index: usize,
    name: &'static str,        // "The Present", "The Crossing", ...
    meaning_key: String,       // key into position-context text
    layout: LayoutSlot,        // grid coords + rotation (the crossing card is +90°)
}

trait Spread {
    fn positions(&self) -> &[PositionDef];
    fn name(&self) -> &str;
}
```

Celtic Cross is the first `impl`. Adding Three-Card, Horseshoe, etc. later = a new
`PositionDef` list + layout, no engine changes.

**Celtic Cross 10 positions** (the cross + the staff):

1. The Present / Significator (center)
2. The Challenge (laid across #1, rotated 90°)
3. The Foundation (below)
4. The Past (left)
5. The Crown / possible outcome (above)
6. The Near Future (right)
7. Self (staff, bottom)
8. Environment (staff)
9. Hopes & Fears (staff)
10. Outcome (staff, top)

### Interpretation (hybrid)

- **Base layer (always on, offline):** `meanings.ron` holds, per `CardId`, an
  `upright` and `reversed` text, plus per-position framing text. The reading
  composes: card meaning × orientation × position context. Deterministic, no
  network, no key.
- **Optional AI layer:** a "Get a deeper reading" button assembles the full drawn
  spread into a prompt and calls Claude for a flowing, woven narrative. Only
  enabled when `ANTHROPIC_API_KEY` is set (or entered in settings).

---

## Claude API integration (Rust specifics)

There is **no official Anthropic Rust SDK**, so we call the REST API directly
with `reqwest` (async, `rustls` TLS) — this is the supported pattern for
languages without an SDK.

- Endpoint: `POST https://api.anthropic.com/v1/messages`
- Headers: `x-api-key: $ANTHROPIC_API_KEY`, `anthropic-version: 2023-06-01`,
  `content-type: application/json`
- Model: **`claude-opus-4-8`** (default; expose a setting to pick sonnet/haiku)
- `max_tokens`: ~4000 for a reading; **stream** the response so long output
  doesn't hit timeouts and the narrative can type out on screen
- `thinking: { "type": "adaptive" }` for a more considered synthesis
- Bridge async → Bevy: spawn the request on a `tokio` runtime / `AsyncComputeTaskPool`
  task and poll the `Task` from a Bevy system, feeding deltas into the UI.

Request body shape:

```json
{
  "model": "claude-opus-4-8",
  "max_tokens": 4000,
  "stream": true,
  "thinking": { "type": "adaptive" },
  "system": "You are a thoughtful Tarot de Marseille reader. Interpret the spread holistically...",
  "messages": [
    {
      "role": "user",
      "content": "Spread: Celtic Cross. Question: <q>. Cards: 1. The Present — Le Mat (upright); 2. The Challenge — XIII (reversed); ... Weave these into a coherent reading."
    }
  ]
}
```

Security: never hardcode the key; read from env or OS keyring. Make the AI path
fully optional so the app is useful with zero configuration.

---

## Animation flow (Bevy states)

```
MainMenu → SpreadSelect → Shuffle → Deal → Reveal → Reading → (DeepReading)
```

- **Shuffle:** deck stacked center; riffle/cut animation (sprites interleave +
  small random offsets), looping briefly. `bevy_tweening` sequences.
- **Deal:** for each position in order, tween a face-down card from the deck along
  an eased arc to its `LayoutSlot` transform; the crossing card lands rotated 90°.
- **Reveal:** flip each card — scale X to 0, swap texture face-down→face-up, scale
  X back to 1 (fake 3D flip). Optional glow pulse on the significator.
- **Reading:** click/hover a card → panel shows position name + composed meaning.
- **DeepReading:** stream Claude's narrative into a scrollable panel.
  //! Procedurally generated textures — no external art needed, so they stay
  //! self-contained and render fine under software rasterization.
  //!
  //! Everything here writes straight RGBA8 (sRGB) byte buffers and wraps them in
  //! a Bevy [`Image`]. Three pieces:
  //! _ [`vignette_image`] — the felt background with a warm center glow and
  //! darkened edges, so the table reads as lit from above.
  //! _ [`soft_shadow_image`] — a reusable soft-edged dark blob placed under each
  //! card for depth.
  //! \* [`card_back_image`] — the ornamental Tarot card back (deep indigo, a
  //! double gold frame, and a single central star motif). Restrained on
  //! purpose: a clean back beats a busy one.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::theme::Rgb;

/// A tiny RGBA8 canvas with alpha-blended drawing helpers.
struct Canvas {
w: usize,
h: usize,
px: Vec<u8>,
}

impl Canvas {
fn new(w: usize, h: usize) -> Self {
Self { w, h, px: vec![0; w * h * 4] }
}

    /// Source-over alpha blend of `(r,g,b)` at coverage `a` (0..=1) onto a pixel.
    fn blend(&mut self, x: usize, y: usize, r: f32, g: f32, b: f32, a: f32) {
        if x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        let a = a.clamp(0.0, 1.0);
        let i = (y * self.w + x) * 4;
        let bg = &self.px[i..i + 4];
        let (br, bg_, bb, ba) = (
            bg[0] as f32 / 255.0,
            bg[1] as f32 / 255.0,
            bg[2] as f32 / 255.0,
            bg[3] as f32 / 255.0,
        );
        let out_a = a + ba * (1.0 - a);
        let mix = |s: f32, d: f32| {
            if out_a <= 0.0 {
                0.0
            } else {
                (s * a + d * ba * (1.0 - a)) / out_a
            }
        };
        self.px[i] = (mix(r, br) * 255.0).round() as u8;
        self.px[i + 1] = (mix(g, bg_) * 255.0).round() as u8;
        self.px[i + 2] = (mix(b, bb) * 255.0).round() as u8;
        self.px[i + 3] = (out_a * 255.0).round() as u8;
    }

    /// Fill the whole canvas with an opaque colour.
    fn fill(&mut self, r: f32, g: f32, b: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                self.blend(x, y, r, g, b, 1.0);
            }
        }
    }

    fn into_image(self) -> Image {
        Image::new(
            Extent3d { width: self.w as u32, height: self.h as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            self.px,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
    }

}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
t _ t _ (3.0 - 2.0 \* t)
}

/// Signed coverage of a rounded rectangle centred in `(w,h)`: 1 inside, a soft
/// edge across `feather` pixels, 0 outside. `inset` shrinks the rect from the
/// canvas edge; `radius` rounds the corners.
fn rounded_rect_coverage(
x: f32,
y: f32,
w: f32,
h: f32,
inset: f32,
radius: f32,
feather: f32,
) -> f32 {
let cx = w / 2.0;
let cy = h / 2.0;
let half_w = (w / 2.0 - inset - radius).max(0.0);
let half_h = (h / 2.0 - inset - radius).max(0.0);
let dx = (x - cx).abs() - half_w;
let dy = (y - cy).abs() - half_h;
// Distance to the rounded-rect boundary (negative inside).
let outside = ((dx.max(0.0)).powi(2) + (dy.max(0.0)).powi(2)).sqrt();
let inside = dx.max(dy).min(0.0);
let dist = outside + inside - radius;
1.0 - smoothstep(-feather, feather, dist)
}

/// The felt background: a warm pool of light at the centre fading to a dark,
/// slightly cool vignette at the edges. `center`/`edge` come from the theme.
pub fn vignette_image(w: usize, h: usize, center: Rgb, edge: Rgb) -> Image {
let mut c = Canvas::new(w, h);
let cx = w as f32 / 2.0;
let cy = h as f32 / 2.0;
let max_r = (cx _ cx + cy _ cy).sqrt();

    // Base felt + radial gradient (center -> edge).
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let r = (dx * dx + dy * dy).sqrt() / max_r;
            // Ease the falloff so the lit pool is broad and the corners drop off.
            let t = smoothstep(0.0, 1.05, r).powf(1.3);
            let rr = center.0 + (edge.0 - center.0) * t;
            let gg = center.1 + (edge.1 - center.1) * t;
            let bb = center.2 + (edge.2 - center.2) * t;
            c.blend(x, y, rr, gg, bb, 1.0);
        }
    }
    c.into_image()

}

/// A reusable soft shadow: a dark, blurred rounded rectangle on transparency.
/// Drawn under a card (scaled to its size) to lift it off the table.
pub fn soft_shadow_image(size: usize) -> Image {
let mut c = Canvas::new(size, size);
let s = size as f32;
for y in 0..size {
for x in 0..size {
// A generously feathered rounded rect, inset so the blur has room.
let cov = rounded_rect_coverage(
x as f32 + 0.5,
y as f32 + 0.5,
s,
s,
s _ 0.16,
s _ 0.10,
s _ 0.16,
);
// Slightly more than coverage near the core for a denser center.
let a = (cov _ 0.55).clamp(0.0, 0.55);
c.blend(x, y, 0.0, 0.0, 0.0, a);
}
}
c.into_image()
}

/// A white, soft-edged rounded glow — tinted at the sprite to colour it (e.g.
/// the gold selection halo behind the active card).
pub fn glow_image(size: usize) -> Image {
let mut c = Canvas::new(size, size);
let s = size as f32;
for y in 0..size {
for x in 0..size {
let cov = rounded_rect_coverage(
x as f32 + 0.5,
y as f32 + 0.5,
s,
s,
s _ 0.22,
s _ 0.14,
s \* 0.20,
);
if cov > 0.0 {
c.blend(x, y, 1.0, 1.0, 1.0, cov);
}
}
}
c.into_image()
}

/// The reading-panel background: a translucent dark rounded card with a thin
/// gold hairline border. `fill`/`gold` come from the theme.
pub fn panel_image(w: usize, h: usize, fill: Rgb, gold: Rgb) -> Image {
let mut c = Canvas::new(w, h);
let fw = w as f32;
let fh = h as f32;
let radius = 22.0;
for y in 0..h {
for x in 0..w {
let fx = x as f32 + 0.5;
let fy = y as f32 + 0.5;
let cov = rounded_rect_coverage(fx, fy, fw, fh, 1.0, radius, 1.2);
if cov > 0.0 {
c.blend(x, y, fill.0, fill.1, fill.2, cov _ 0.92);
}
// Gold hairline just inside the edge.
let outer = rounded_rect_coverage(fx, fy, fw, fh, 2.0, radius, 0.8);
let inner = rounded_rect_coverage(fx, fy, fw, fh, 3.6, radius, 0.8);
let band = (outer - inner).clamp(0.0, 1.0);
if band > 0.0 {
c.blend(x, y, gold.0, gold.1, gold.2, band _ 0.6);
}
}
}
c.into_image()
}

/// A small badge disc for position numbers: a dark fill ringed in gold. Colours
/// come from the theme.
pub fn disc_image(size: usize, gold: Rgb, fill: Rgb) -> Image {
let mut c = Canvas::new(size, size);
let s = size as f32;
let cx = s / 2.0;
let cy = s / 2.0;
let r = s / 2.0 - 1.0;
let ring = s _ 0.10;
for y in 0..size {
for x in 0..size {
let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
let disc = 1.0 - smoothstep(r - 1.0, r + 0.5, d);
if disc <= 0.0 {
continue;
}
// Gold near the rim, dark fill inside.
let is_ring = smoothstep(r - ring - 1.0, r - ring + 1.0, d);
let rr = fill.0 + (gold.0 - fill.0) _ is_ring;
let gg = fill.1 + (gold.1 - fill.1) _ is_ring;
let bb = fill.2 + (gold.2 - fill.2) _ is_ring;
c.blend(x, y, rr, gg, bb, disc);
}
}
c.into_image()
}

/// The ornamental card back. A deep `field`, a faint diagonal lattice, a double
/// gold frame, and a single eight-point star at the centre. Colours come from
/// the theme.
pub fn card_back_image(w: usize, h: usize, field: Rgb, gold: Rgb) -> Image {
let mut c = Canvas::new(w, h);
let fw = w as f32;
let fh = h as f32;

    // Field: deep, faintly darker toward the edges.
    c.fill(field.0, field.1, field.2);

    // Faint diagonal lattice for texture (very low alpha gold).
    let spacing = (fw / 7.0).max(8.0);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32;
            let fy = y as f32;
            let a = (((fx + fy) / spacing * std::f32::consts::PI).sin().abs()).powf(8.0)
                + (((fx - fy) / spacing * std::f32::consts::PI).sin().abs()).powf(8.0);
            if a > 0.02 {
                c.blend(x, y, gold.0, gold.1, gold.2, a * 0.05);
            }
        }
    }

    // Mask the lattice/field to a rounded-rect card shape (transparent corners).
    let radius = fw * 0.07;
    for y in 0..h {
        for x in 0..w {
            let cov = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                fw,
                fh,
                0.0,
                radius,
                1.2,
            );
            if cov < 0.999 {
                let i = (y * w + x) * 4;
                c.px[i + 3] = (c.px[i + 3] as f32 * cov) as u8;
            }
        }
    }

    // Double gold frame (outer thick, inner thin), following the rounded shape.
    let draw_frame = |c: &mut Canvas, inset: f32, thickness: f32, alpha: f32| {
        for y in 0..h {
            for x in 0..w {
                let fx = x as f32 + 0.5;
                let fy = y as f32 + 0.5;
                let outer =
                    rounded_rect_coverage(fx, fy, fw, fh, inset, radius * 0.8, 0.8);
                let inner = rounded_rect_coverage(
                    fx,
                    fy,
                    fw,
                    fh,
                    inset + thickness,
                    radius * 0.8,
                    0.8,
                );
                let band = (outer - inner).clamp(0.0, 1.0);
                if band > 0.0 {
                    c.blend(x, y, gold.0, gold.1, gold.2, band * alpha);
                }
            }
        }
    };
    draw_frame(&mut c, fw * 0.06, fw * 0.018, 0.95);
    draw_frame(&mut c, fw * 0.11, fw * 0.010, 0.75);

    // Central eight-point star: two overlaid squares (a square + its 45° twin),
    // rendered as a filled star polygon via angular radius modulation.
    let cx = fw / 2.0;
    let cy = fh / 2.0;
    let r_out = fw * 0.20;
    let r_in = r_out * 0.42;
    let points = 8.0;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx);
            // Star radius at this angle (cusped between r_in and r_out).
            let phase = (ang * points / 2.0).cos().abs();
            let r_edge = r_in + (r_out - r_in) * phase;
            let cov = 1.0 - smoothstep(r_edge - 1.2, r_edge + 1.2, dist);
            if cov > 0.0 {
                c.blend(x, y, gold.0, gold.1, gold.2, cov * 0.92);
            }
        }
    }
    // A small dark center dot inside the star for definition.
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let cov = 1.0 - smoothstep(r_in * 0.5 - 1.0, r_in * 0.5 + 1.0, dist);
            if cov > 0.0 {
                c.blend(x, y, field.0, field.1, field.2, cov);
            }
        }
    }

    c.into_image()

}

Respect a "reduced motion" setting (instant placement) for accessibility.

---

## Crate list (initial)

| Crate                | Purpose                                  |
| -------------------- | ---------------------------------------- |
| `bevy`               | engine, rendering, ECS, states           |
| `bevy_tweening`      | declarative tweens for deal/flip/glow    |
| `rand`               | shuffle + orientation                    |
| `ron`                | data files (meanings, spreads)           |
| `serde`              | (de)serialize domain + data              |
| `reqwest`            | Claude REST calls (rustls, json, stream) |
| `tokio`              | async runtime for the AI task            |
| `anyhow`/`thiserror` | error handling                           |
| `keyring` (opt)      | store API key in OS secret store         |

Pin Bevy and `bevy_tweening` to a compatible pair at project start (Bevy's API
moves fast; verify the `bevy_tweening` version matches the chosen Bevy version).

---

## Roadmap (MVP-first)

**Phase 0 — skeleton**

- Cargo project, Bevy window opens, asset dir wired, CI build on Linux.

**Phase 1 — domain core (no UI)**

- Full 78-card deck, shuffle + draw, Celtic Cross positions, meanings loader.
- Unit tests: deck has 78 unique cards; draw is non-repeating; every `CardId`
  resolves to art + meaning.

**Phase 2 — static render** ✅ _done_

- Real public-domain TdM scans (Wikimedia) laid out in the 10 Celtic Cross
  positions, crossing card rotated. No animation.

**Phase 3 — animation + theming** ✅ _done_

- Hand-rolled tween system instead of `bevy_tweening` (which trails two Bevy
  versions — incompatible with 0.18). Deck-stack → deal-along-arc → staggered
  flip-reveal; reversed cards rendered upside-down. Procedural textures (felt
  vignette, ornamental card back, soft shadows, badge discs, gold selection
  glow, panel). Themed UI (Liberation Serif): title, numbered position badges,
  a reading panel gated so no card's meaning shows before its flip completes.
  Click-to-select a card; `Space` redeals; `R` toggles reduced motion
  (`TARO_REDUCED_MOTION` env also forces it). Deterministic screenshot harness
  (`TARO_CAPTURE`/`TARO_CAPTURE_AT`) for verifying visuals headlessly.

**Phase 4 — reading UI** ✅ _done_

- In-app **question input** (top banner; `Enter` to focus, `TARO_QUESTION` to
  seed) and a **woven full-reading overlay** (`Tab`) that composes the whole
  spread into one flowing narrative — `taro_domain::compose_reading` (pure,
  offline, tested), shown over a dimming scrim and gated by the same no-spoiler
  rule as the panel. The per-card panel + selection (Phase 3) remain. This is
  the first genuinely usable release.

**Phase 5 — AI deeper reading**

- `reqwest` streaming call to Claude, async→Bevy bridge, settings for the key.

**Phase 6 — polish & packaging**

- More spreads behind the `Spread` trait, sound, theming, AppImage + Flatpak,
  bundle assets.

---

## Open items / risks

- **Asset sourcing & licensing:** confirm the chosen TdM deck edition is truly
  public domain; clean/upscale ~78 scans to a consistent size. This is the main
  art-pipeline effort and gates Phase 2's final look.
- **Bevy version churn:** lock versions early; `bevy_tweening` must match.
- **async↔ECS bridge:** the AI streaming-into-UI path is the trickiest plumbing;
  isolate it in `ai.rs` and keep the rest synchronous.
- **Divination framing:** present readings as reflective/entertainment, not
  predictive fact (worth a small disclaimer in-app).
