# Taro — Animated Tarot de Marseille Fortune Telling

A Linux desktop app (Rust) that performs animated tarot readings using the
Tarot de Marseille, starting with the Celtic Cross spread.

## Decisions (locked)

| Area            | Choice                                                                 |
|-----------------|-----------------------------------------------------------------------|
| Language        | Rust (2021 edition)                                                    |
| Rendering / UI  | **Bevy** (2D ECS game engine) + `bevy_tweening` for card animation    |
| Interpretation  | **Hybrid** — authored static meanings (offline) + optional Claude API |
| Card art        | **Public-domain TdM scans** (e.g. Jean Dodal 1701 / Conver 1760), 78  |
| Target          | Linux desktop; package as AppImage and Flatpak                        |

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
    { "role": "user", "content": "Spread: Celtic Cross. Question: <q>. Cards: 1. The Present — Le Mat (upright); 2. The Challenge — XIII (reversed); ... Weave these into a coherent reading." }
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

Respect a "reduced motion" setting (instant placement) for accessibility.

---

## Crate list (initial)

| Crate           | Purpose                                  |
|-----------------|------------------------------------------|
| `bevy`          | engine, rendering, ECS, states           |
| `bevy_tweening` | declarative tweens for deal/flip/glow    |
| `rand`          | shuffle + orientation                    |
| `ron`           | data files (meanings, spreads)           |
| `serde`         | (de)serialize domain + data              |
| `reqwest`       | Claude REST calls (rustls, json, stream) |
| `tokio`         | async runtime for the AI task            |
| `anyhow`/`thiserror` | error handling                      |
| `keyring` (opt) | store API key in OS secret store         |

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

**Phase 4 — reading UI**
- Question input, full composed reading text (the panel + selection already
  cover per-card meanings). This is the first genuinely usable release.

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
