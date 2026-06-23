//! Shared visual language: palette, fonts, and window/layout constants.

use bevy::prelude::*;

// ── Window ──────────────────────────────────────────────────────────────────
pub const WIN_W: f32 = 1280.0;
pub const WIN_H: f32 = 860.0;

// ── Palette ─────────────────────────────────────────────────────────────────
/// Warm gold — frames, headings, the significator glow.
pub const GOLD: Color = Color::srgb(0.83, 0.69, 0.38);
/// Dimmer gold for secondary strokes and inactive accents.
pub const GOLD_DIM: Color = Color::srgb(0.52, 0.44, 0.27);
/// Aged-parchment off-white for body text.
pub const PARCHMENT: Color = Color::srgb(0.90, 0.86, 0.77);
/// Muted lavender-grey for captions and labels.
pub const MUTED: Color = Color::srgb(0.62, 0.58, 0.70);

// ── Fonts ───────────────────────────────────────────────────────────────────
pub const FONT_REGULAR: &str = "fonts/LiberationSerif-Regular.ttf";
pub const FONT_BOLD: &str = "fonts/LiberationSerif-Bold.ttf";
pub const FONT_ITALIC: &str = "fonts/LiberationSerif-Italic.ttf";
