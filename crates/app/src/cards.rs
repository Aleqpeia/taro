//! Spawning the dealt cards: each card face, its drop shadow, and a numbered
//! position badge. Cards deal from a face-down deck, fly to their slots, and
//! flip to reveal — or, under reduced motion, simply appear at rest face up.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::f32::consts::FRAC_PI_2;
use taro_domain::{LayoutSlot, Orientation};

use crate::layout::{card_rotation, slot_translation, CARD_H, CARD_W};
use crate::theme::Themed;
use crate::tween::{Ease, FadeIn, FlipReveal, TransformTween};
use crate::{Fonts, Textures};

// ── Deal timing (seconds, relative to the deal's start `t0`) ──────────────────
/// Where the face-down deck waits before dealing (world space).
const DECK: Vec2 = Vec2::new(-210.0, 24.0);
const DEAL_START: f32 = 0.45;
pub const DEAL_STAGGER: f32 = 0.12;
const DEAL_DUR: f32 = 0.55;
const DEAL_ARC: f32 = 58.0;
pub const REVEAL_BASE: f32 = 2.35;
pub const REVEAL_STAGGER: f32 = 0.16;
const FLIP_HALF: f32 = 0.16;

/// When card `index`'s face has finished flipping up (used to gate the panel).
pub fn reveal_done_time(index: usize) -> f32 {
    REVEAL_BASE + index as f32 * REVEAL_STAGGER + 2.0 * FLIP_HALF
}

/// Tags every entity belonging to the current deal, so a redeal can clear them.
#[derive(Component)]
pub struct SpreadEntity;

/// Marks a dealt card and records what the interaction/animation layers need.
#[derive(Component)]
pub struct CardView {
    /// Index into the reading's entries (and the drawn-card list).
    pub index: usize,
    /// Where the card comes to rest, face up.
    pub rest: Transform,
}

/// Screen-space offset of the numbered badge from the card centre. The crossing
/// card uses the top-right corner so it doesn't collide with card 1's badge.
fn badge_offset(slot: LayoutSlot) -> Vec2 {
    if slot.rotated {
        Vec2::new(CARD_W * 0.5 - 13.0, CARD_H * 0.5 - 13.0)
    } else {
        Vec2::new(-CARD_W * 0.5 + 13.0, CARD_H * 0.5 - 13.0)
    }
}

/// The z-depth a card rests at; the crossing card sits above its partner.
fn card_z(slot: LayoutSlot, index: usize) -> f32 {
    if slot.rotated {
        6.0
    } else {
        3.0 + index as f32 * 0.05
    }
}

/// Spawn one card (shadow + face + badge), animated unless `reduced`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_card(
    commands: &mut Commands,
    textures: &Textures,
    fonts: &Fonts,
    gold: Color,
    face: Handle<Image>,
    slot: LayoutSlot,
    orientation: Orientation,
    position_index: usize,
    index: usize,
    reduced: bool,
    t0: f32,
) {
    let pos = slot_translation(slot);
    let z = card_z(slot, index);
    let rest = Transform {
        translation: pos.extend(z),
        rotation: card_rotation(slot, orientation),
        ..default()
    };

    let deal_at = t0 + DEAL_START + index as f32 * DEAL_STAGGER;
    let land_at = deal_at + DEAL_DUR;
    let reveal_at = t0 + REVEAL_BASE + index as f32 * REVEAL_STAGGER;
    let shadow_rot = Quat::from_rotation_z(if slot.rotated { FRAC_PI_2 } else { 0.0 });

    // ── Drop shadow ──────────────────────────────────────────────────────────
    let shadow_tf = Transform {
        translation: (pos + Vec2::new(8.0, -12.0)).extend(z - 1.0),
        rotation: shadow_rot,
        ..default()
    };
    let mut shadow = commands.spawn((
        Sprite {
            image: textures.shadow.clone(),
            custom_size: Some(Vec2::new(CARD_W * 1.24, CARD_H * 1.16)),
            color: Color::WHITE.with_alpha(if reduced { 1.0 } else { 0.0 }),
            ..default()
        },
        shadow_tf,
        SpreadEntity,
    ));
    if !reduced {
        shadow.insert(FadeIn { start_time: land_at - 0.2, duration: 0.3 });
    }

    // ── Card ─────────────────────────────────────────────────────────────────
    let (initial_image, start_tf) = if reduced {
        (face.clone(), rest)
    } else {
        // Start as a face-down card in the deck, slightly small and squared up.
        let start = Transform {
            translation: (DECK + Vec2::new(index as f32 * 1.3, index as f32 * 0.8)).extend(z),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(0.92),
        };
        (textures.card_back.clone(), start)
    };
    let mut card = commands.spawn((
        Sprite {
            image: initial_image,
            custom_size: Some(Vec2::new(CARD_W, CARD_H)),
            ..default()
        },
        start_tf,
        CardView { index, rest },
        SpreadEntity,
    ));
    if !reduced {
        card.insert(TransformTween::new(start_tf, rest, deal_at, DEAL_DUR, Ease::OutCubic).with_arc(DEAL_ARC));
        card.insert(FlipReveal {
            start_time: reveal_at,
            half: FLIP_HALF,
            face: face.clone(),
            swapped: false,
            done: false,
        });
    }

    // ── Numbered badge (disc + number, screen-upright) ───────────────────────
    let badge_pos = (pos + badge_offset(slot)).extend(20.0);
    let badge_fade = FadeIn { start_time: reveal_at + 0.1, duration: 0.3 };
    let badge_alpha = if reduced { 1.0 } else { 0.0 };

    let mut disc = commands.spawn((
        Sprite {
            image: textures.disc.clone(),
            custom_size: Some(Vec2::splat(26.0)),
            color: Color::WHITE.with_alpha(badge_alpha),
            ..default()
        },
        Transform::from_translation(badge_pos),
        SpreadEntity,
    ));
    if !reduced {
        disc.insert(FadeIn { ..badge_fade });
    }

    let mut num = commands.spawn((
        Text2d::new(position_index.to_string()),
        TextFont { font: fonts.bold.clone(), font_size: 15.0, ..default() },
        TextColor(gold.with_alpha(badge_alpha)),
        Themed::Gold,
        Anchor::CENTER,
        Transform::from_translation(badge_pos + Vec3::new(0.0, 0.0, 0.1)),
        SpreadEntity,
    ));
    if !reduced {
        num.insert(badge_fade);
    }
}
