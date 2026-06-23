//! Shared visual language: palette presets, fonts, and design constants.
//!
//! The palette lives in a [`Theme`] resource rather than as bare constants, so
//! the whole look — text, accents, and the procedurally generated felt, card
//! back, panel, and badges — can be swapped at runtime. Presets are listed in
//! [`THEMES`]; one is chosen at startup (env `TARO_THEME`) and cycled with `T`.

use bevy::prelude::*;

// ── Design resolution ─────────────────────────────────────────────────────────
// The layout is authored against this fixed "design" canvas; a fit-to-window
// camera (`ScalingMode::AutoMin`) scales it to any real window size, so these
// are reference dimensions, not a hard cap on the window.
pub const WIN_W: f32 = 1280.0;
pub const WIN_H: f32 = 860.0;

// ── Fonts ───────────────────────────────────────────────────────────────────
pub const FONT_REGULAR: &str = "fonts/LiberationSerif-Regular.ttf";
pub const FONT_BOLD: &str = "fonts/LiberationSerif-Bold.ttf";
pub const FONT_ITALIC: &str = "fonts/LiberationSerif-Italic.ttf";

/// An sRGB triple in `0..=1`, used both as a `Color` (for text/sprites) and as
/// raw bytes by the procedural texture generators in `textures.rs`.
pub type Rgb = (f32, f32, f32);

fn col((r, g, b): Rgb) -> Color {
    Color::srgb(r, g, b)
}

/// A complete palette. One theme drives every colour in the scene, so swapping
/// it keeps the look coherent — accents, body text, the felt vignette, the card
/// back's field, and the reading panel all move together.
#[derive(Resource, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    /// Warm accent — frames, headings, the significator glow.
    pub gold: Rgb,
    /// Dimmer accent for secondary strokes and inactive labels.
    pub gold_dim: Rgb,
    /// Body-text off-white.
    pub parchment: Rgb,
    /// Captions and labels.
    pub muted: Rgb,
    /// Orientation tag when a card falls reversed.
    pub reversed: Rgb,
    /// Felt vignette: lit centre and shadowed edge.
    pub felt_center: Rgb,
    pub felt_edge: Rgb,
    /// The card back's field colour.
    pub card_field: Rgb,
    /// The reading panel's translucent fill.
    pub panel_fill: Rgb,
}

impl Theme {
    pub fn gold(&self) -> Color { col(self.gold) }
    pub fn gold_dim(&self) -> Color { col(self.gold_dim) }
    pub fn parchment(&self) -> Color { col(self.parchment) }
    pub fn muted(&self) -> Color { col(self.muted) }
    pub fn reversed(&self) -> Color { col(self.reversed) }
    /// Window clear colour — matches the felt edge so letterbox margins blend in.
    pub fn clear(&self) -> Color { col(self.felt_edge) }
}

/// Built-in presets, in cycle order. The first is the default.
pub const THEMES: &[Theme] = &[
    // Midnight — the original: lit indigo felt, warm gold.
    Theme {
        name: "midnight",
        gold: (0.83, 0.69, 0.38),
        gold_dim: (0.52, 0.44, 0.27),
        parchment: (0.90, 0.86, 0.77),
        muted: (0.62, 0.58, 0.70),
        reversed: (0.78, 0.52, 0.46),
        felt_center: (0.115, 0.085, 0.155),
        felt_edge: (0.030, 0.024, 0.052),
        card_field: (0.105, 0.090, 0.205),
        panel_fill: (0.085, 0.068, 0.125),
    },
    // Emerald — a deep green baize, gold accents.
    Theme {
        name: "emerald",
        gold: (0.84, 0.71, 0.40),
        gold_dim: (0.50, 0.46, 0.30),
        parchment: (0.90, 0.88, 0.79),
        muted: (0.58, 0.66, 0.60),
        reversed: (0.82, 0.56, 0.45),
        felt_center: (0.060, 0.130, 0.100),
        felt_edge: (0.012, 0.035, 0.027),
        card_field: (0.060, 0.120, 0.100),
        panel_fill: (0.050, 0.100, 0.082),
    },
    // Wine — burgundy felt, brighter gilt.
    Theme {
        name: "wine",
        gold: (0.88, 0.74, 0.44),
        gold_dim: (0.58, 0.42, 0.30),
        parchment: (0.93, 0.86, 0.80),
        muted: (0.72, 0.56, 0.58),
        reversed: (0.86, 0.62, 0.46),
        felt_center: (0.165, 0.055, 0.075),
        felt_edge: (0.048, 0.012, 0.020),
        card_field: (0.150, 0.050, 0.080),
        panel_fill: (0.110, 0.040, 0.060),
    },
    // Ash — cool slate monochrome with silver accents.
    Theme {
        name: "ash",
        gold: (0.76, 0.80, 0.86),
        gold_dim: (0.46, 0.50, 0.56),
        parchment: (0.88, 0.90, 0.93),
        muted: (0.58, 0.62, 0.68),
        reversed: (0.80, 0.62, 0.62),
        felt_center: (0.130, 0.140, 0.170),
        felt_edge: (0.035, 0.038, 0.050),
        card_field: (0.120, 0.130, 0.170),
        panel_fill: (0.100, 0.110, 0.140),
    },
];

/// The index of the active theme; bumped by the `T` key to cycle [`THEMES`].
#[derive(Resource)]
pub struct ThemeIndex(pub usize);

/// Resolve a preset index by name (case-insensitive), for the `TARO_THEME` env.
pub fn theme_index_by_name(name: &str) -> Option<usize> {
    THEMES.iter().position(|t| t.name.eq_ignore_ascii_case(name))
}

/// A text or sprite whose colour follows a palette role, so the runtime
/// theme-swap system can recolour it. Dynamic panel text (set per selection by
/// `update_panel`) is intentionally not tagged.
#[derive(Component, Clone, Copy)]
pub enum Themed {
    Gold,
    GoldDim,
    Parchment,
    Muted,
    /// A hairline divider: dimmed gold at half alpha.
    Divider,
}

impl Themed {
    pub fn color(&self, t: &Theme) -> Color {
        match self {
            Themed::Gold => t.gold(),
            Themed::GoldDim => t.gold_dim(),
            Themed::Parchment => t.parchment(),
            Themed::Muted => t.muted(),
            Themed::Divider => t.gold_dim().with_alpha(0.5),
        }
    }
}
