//! Maps a spread's abstract grid [`LayoutSlot`] (col/row) to on-screen world
//! coordinates. Pure presentation — no domain logic.
//!
//! Bevy's 2D world has +Y up and the origin at the screen center. The spread is
//! shifted left to leave room for the reading panel on the right.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
use taro_domain::{LayoutSlot, Orientation};

/// Card display size in pixels (≈ the 0.53 aspect of the scans).
pub const CARD_W: f32 = 104.0;
pub const CARD_H: f32 = 188.0;

/// Center-to-center spacing between adjacent slots.
pub const COL_SPACING: f32 = CARD_W + 30.0;
pub const ROW_SPACING: f32 = CARD_H + 12.0;

/// Horizontal shift of the whole spread, opening space for the right panel.
pub const SPREAD_X: f32 = -210.0;
/// Vertical nudge so the spread sits a touch below the title.
pub const SPREAD_Y: f32 = -16.0;

/// Translate a grid slot into a world-space position (z left to the caller).
pub fn slot_translation(slot: LayoutSlot) -> Vec2 {
    let x = (slot.col as f32 - 2.0) * COL_SPACING + SPREAD_X;
    let y = (1.5 - slot.row as f32) * ROW_SPACING + SPREAD_Y;
    Vec2::new(x, y)
}

/// Resting rotation for a card: the crossing card lies sideways (90°), and a
/// reversed card is shown upside down (+180°) — the orientation made visible.
pub fn card_rotation(slot: LayoutSlot, orientation: Orientation) -> Quat {
    let mut a = if slot.rotated { FRAC_PI_2 } else { 0.0 };
    if matches!(orientation, Orientation::Reversed) {
        a += PI;
    }
    Quat::from_rotation_z(a)
}
