//! Input: redeal, reduced-motion toggle, click-to-select, and the gold
//! selection highlight.

use bevy::prelude::*;

use crate::cards::{reveal_done_time, CardView, SpreadEntity};
use crate::layout::{CARD_H, CARD_W};
use crate::theme::Theme;
use crate::tween::{FlipReveal, TransformTween};
use crate::{clear_spread, deal, DealInfo, DebugRedeal, Fonts, Motion, Selected, Textures};

/// Space deals a new reading; `R` toggles reduced motion and redeals.
#[allow(clippy::too_many_arguments)]
pub fn redeal_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    textures: Res<Textures>,
    fonts: Res<Fonts>,
    assets: Res<AssetServer>,
    time: Res<Time>,
    theme: Res<Theme>,
    mut motion: ResMut<Motion>,
    mut selected: ResMut<Selected>,
    spread: Query<Entity, With<SpreadEntity>>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyR);
    if toggle {
        motion.reduced = !motion.reduced;
    }
    if toggle || keys.just_pressed(KeyCode::Space) {
        clear_spread(&mut commands, &spread);
        selected.0 = 0;
        deal(&mut commands, &textures, &fonts, &assets, &theme, motion.reduced, time.elapsed_secs());
    }
}

/// Debug-only: fire exactly one redeal at `TARO_REDEAL_AT`, sharing the real
/// redeal code path so the harness can verify clear + re-deal.
#[allow(clippy::too_many_arguments)]
pub fn debug_redeal(
    mut commands: Commands,
    time: Res<Time>,
    mut dr: ResMut<DebugRedeal>,
    textures: Res<Textures>,
    fonts: Res<Fonts>,
    assets: Res<AssetServer>,
    theme: Res<Theme>,
    motion: Res<Motion>,
    mut selected: ResMut<Selected>,
    spread: Query<Entity, With<SpreadEntity>>,
) {
    let Some(at) = dr.at else {
        return;
    };
    if dr.done || time.elapsed_secs() < at {
        return;
    }
    dr.done = true;
    clear_spread(&mut commands, &spread);
    selected.0 = 0;
    deal(&mut commands, &textures, &fonts, &assets, &theme, motion.reduced, time.elapsed_secs());
}

/// Left-click selects the card under the cursor (topmost wins).
pub fn select_input(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    cards: Query<(&CardView, &GlobalTransform)>,
    mut selected: ResMut<Selected>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (cam, cam_tf) = *camera;
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let candidates: Vec<(usize, GlobalTransform)> =
        cards.iter().map(|(c, gt)| (c.index, *gt)).collect();
    if let Some(index) = card_at(world, &candidates) {
        selected.0 = index;
    }
}

/// Pure hit-test: the index of the topmost (highest-z) card whose oriented box
/// contains `world`, or `None`. Each card's box is `CARD_W × CARD_H` in its own
/// rotated/scaled frame.
pub fn card_at(world: Vec2, cards: &[(usize, GlobalTransform)]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, gt) in cards {
        let local = gt.affine().inverse().transform_point3(world.extend(0.0));
        if local.x.abs() <= CARD_W * 0.5 && local.y.abs() <= CARD_H * 0.5 {
            let z = gt.translation().z;
            if best.is_none_or(|(_, bz)| z > bz) {
                best = Some((*index, z));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Lift and gold-glow the selected card once it has settled face-up.
#[allow(clippy::type_complexity)]
pub fn apply_selection_highlight(
    selected: Res<Selected>,
    time: Res<Time>,
    theme: Res<Theme>,
    deal_info: Option<Res<DealInfo>>,
    mut cards: Query<(
        &CardView,
        &mut Transform,
        Option<&TransformTween>,
        Option<&FlipReveal>,
    )>,
    mut ring: Query<(&mut Transform, &mut Sprite), (With<Highlight>, Without<CardView>)>,
) {
    let Some(info) = deal_info else {
        return;
    };
    let now = time.elapsed_secs();

    let mut sel_target: Option<(Vec3, Quat)> = None;
    for (card, mut tf, tween, flip) in &mut cards {
        let settled = tween.is_none_or(|t| t.done) && flip.is_none_or(|f| f.done);
        if !settled {
            continue;
        }
        let is_sel = card.index == selected.0;
        let scale = if is_sel { 1.07 } else { 1.0 };
        tf.scale = Vec3::new(scale, scale, 1.0);
        tf.translation = card.rest.translation + if is_sel { Vec3::new(0.0, 6.0, 0.0) } else { Vec3::ZERO };
        if is_sel {
            sel_target = Some((card.rest.translation, card.rest.rotation));
        }
    }

    if let Ok((mut rtf, mut rsprite)) = ring.single_mut() {
        let revealed = info.reduced || now >= info.t0 + reveal_done_time(selected.0);
        match (sel_target, revealed) {
            (Some((pos, rot)), true) => {
                rtf.translation = Vec3::new(pos.x, pos.y + 6.0, pos.z - 0.4);
                rtf.rotation = rot;
                // Gentle breathing glow.
                let pulse = 0.45 + 0.12 * (now * 2.2).sin();
                rsprite.color = theme.gold().with_alpha(pulse);
            }
            _ => rsprite.color = theme.gold().with_alpha(0.0),
        }
    }
}

/// The gold glow sprite behind the selected card.
#[derive(Component)]
pub struct Highlight;

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn at(x: f32, y: f32, z: f32, rot: f32) -> GlobalTransform {
        GlobalTransform::from(Transform {
            translation: Vec3::new(x, y, z),
            rotation: Quat::from_rotation_z(rot),
            ..default()
        })
    }

    #[test]
    fn empty_space_hits_nothing() {
        let cards = [(0usize, at(0.0, 0.0, 3.0, 0.0))];
        assert_eq!(card_at(Vec2::new(500.0, 500.0), &cards), None);
    }

    #[test]
    fn overlapping_cards_pick_topmost_z() {
        // An upright card and a crossing (90°) card share the centre; the
        // crossing card has the higher z, so a central click selects it.
        let cards = [(1usize, at(0.0, 0.0, 3.0, 0.0)), (2usize, at(0.0, 0.0, 6.0, FRAC_PI_2))];
        assert_eq!(card_at(Vec2::ZERO, &cards), Some(2));
    }

    #[test]
    fn point_outside_crossing_card_falls_through_to_upright() {
        // A point high on the upright card is beyond the sideways card's short
        // axis, so only the upright card is hit.
        let cards = [(1usize, at(0.0, 0.0, 3.0, 0.0)), (2usize, at(0.0, 0.0, 6.0, FRAC_PI_2))];
        let p = Vec2::new(0.0, CARD_H * 0.45);
        assert_eq!(card_at(p, &cards), Some(1));
    }
}
