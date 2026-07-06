//! The full-reading overlay: a toggled panel that weaves the whole spread into
//! one flowing narrative (`taro_domain::compose_reading`), the offline
//! counterpart to the future Claude "deeper reading".
//!
//! `Tab` toggles it; the env `TARO_SHOW_READING_AT=secs` opens it once for the
//! headless capture harness. Honouring the panel's no-spoiler rule, the prose is
//! withheld until every card has flipped up.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

use taro_domain::compose_reading;

use crate::ai::{DeepReading, DeepState};
use crate::cards::reveal_done_time;
use crate::question::QuestionInput;
use crate::theme::{Theme, Themed, WIN_H, WIN_W};
use crate::{DealInfo, Fonts, ReadingData, Textures};

const Z_ROOT: f32 = 60.0;
/// The last-dealt card (index 9) flips last, so its reveal time gates the whole.
const LAST_CARD: usize = 9;

/// Whether every card of the current deal has flipped face-up (the no-spoiler
/// gate shared by the overlay and the deeper-reading trigger).
pub fn spread_settled(deal_info: Option<&DealInfo>, now: f32) -> bool {
    deal_info.is_some_and(|info| info.reduced || now >= info.t0 + reveal_done_time(LAST_CARD))
}

/// Whether the full-reading overlay is showing.
#[derive(Resource, Default)]
pub struct ShowFullReading(pub bool);

#[derive(Component)]
pub struct FullReadingRoot;
#[derive(Component)]
pub struct FullReadingScrim;
#[derive(Component)]
pub struct FullReadingHeading;
#[derive(Component)]
pub struct FullReadingBody;
#[derive(Component)]
pub struct FullReadingHint;

/// Spawn the overlay (hidden). Toggled by [`update_full_reading`] via the root's
/// `Visibility`, which the children inherit.
pub fn spawn_full_reading(
    commands: &mut Commands,
    textures: &Textures,
    fonts: &Fonts,
    theme: &Theme,
) {
    commands
        .spawn((Transform::from_xyz(0.0, 0.0, Z_ROOT), Visibility::Hidden, FullReadingRoot))
        .with_children(|p| {
            // Dimming scrim over the whole table (sized to cover any window).
            p.spawn((
                Sprite::from_color(
                    theme.clear().with_alpha(0.92),
                    Vec2::new(WIN_W * 4.0, WIN_H * 4.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.0),
                FullReadingScrim,
            ));
            // Framed panel.
            p.spawn((
                Sprite {
                    image: textures.panel.clone(),
                    custom_size: Some(Vec2::new(980.0, 720.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 1.0),
            ));
            // Heading (the question, or a generic title).
            p.spawn((
                Text2d::new(String::new()),
                TextFont { font: fonts.bold.clone(), font_size: 24.0, ..default() },
                TextColor(theme.gold()),
                Themed::Gold,
                TextLayout::new_with_justify(Justify::Center),
                TextBounds { width: Some(880.0), height: None },
                Anchor::TOP_CENTER,
                Transform::from_xyz(0.0, 332.0, 2.0),
                FullReadingHeading,
            ));
            // Body (woven prose).
            p.spawn((
                Text2d::new(String::new()),
                TextFont { font: fonts.regular.clone(), font_size: 16.0, ..default() },
                TextColor(theme.parchment()),
                Themed::Parchment,
                TextLayout::new_with_justify(Justify::Left),
                TextBounds { width: Some(880.0), height: Some(600.0) },
                Anchor::TOP_CENTER,
                Transform::from_xyz(0.0, 286.0, 2.0),
                FullReadingBody,
            ));
            // Hint / status line for the Claude deeper reading (Phase 5).
            p.spawn((
                Text2d::new(String::new()),
                TextFont { font: fonts.italic.clone(), font_size: 14.0, ..default() },
                TextColor(theme.gold_dim()),
                Themed::GoldDim,
                TextLayout::new_with_justify(Justify::Center),
                TextBounds { width: Some(880.0), height: None },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(0.0, -334.0, 2.0),
                FullReadingHint,
            ));
        });
}

/// `Tab` toggles the overlay (suppressed while typing a question); the env
/// `TARO_SHOW_READING_AT=secs` fires one open for the harness.
pub fn toggle_full_reading(
    keys: Res<ButtonInput<KeyCode>>,
    q: Res<QuestionInput>,
    time: Res<Time>,
    mut at: Local<Option<f32>>,
    mut init: Local<bool>,
    mut show: ResMut<ShowFullReading>,
) {
    if !*init {
        *init = true;
        *at = std::env::var("TARO_SHOW_READING_AT").ok().and_then(|s| s.parse().ok());
    }
    if at.is_some_and(|t| time.elapsed_secs() >= t) {
        *at = None;
        show.0 = true;
        return;
    }
    if !q.editing && keys.just_pressed(KeyCode::Tab) {
        show.0 = !show.0;
    }
}

/// Drive the overlay's visibility and text while it is open.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_full_reading(
    show: Res<ShowFullReading>,
    reading: Option<Res<ReadingData>>,
    q: Res<QuestionInput>,
    deal_info: Option<Res<DealInfo>>,
    time: Res<Time>,
    theme: Res<Theme>,
    deep: Res<DeepReading>,
    mut root: Query<&mut Visibility, With<FullReadingRoot>>,
    mut scrim: Query<&mut Sprite, With<FullReadingScrim>>,
    mut texts: ParamSet<(
        Query<&mut Text2d, With<FullReadingHeading>>,
        Query<&mut Text2d, With<FullReadingBody>>,
        Query<&mut Text2d, With<FullReadingHint>>,
    )>,
) {
    if let Ok(mut vis) = root.single_mut() {
        *vis = if show.0 { Visibility::Visible } else { Visibility::Hidden };
    }
    if !show.0 {
        return;
    }
    if let Ok(mut s) = scrim.single_mut() {
        s.color = theme.clear().with_alpha(0.92);
    }

    // No-spoiler: withhold the woven reading until every card has flipped up.
    let settled = spread_settled(deal_info.as_deref(), time.elapsed_secs());

    let question = (!q.text.trim().is_empty()).then(|| q.text.trim());
    let (heading, mut body) = match (settled, reading.as_ref()) {
        (true, Some(r)) => (
            question.map_or_else(|| "Your reading".to_string(), |s| format!("“{s}”")),
            compose_reading(&r.0, question),
        ),
        _ => (
            "Your reading".to_string(),
            "The cards are still settling — let the spread finish, then press Tab.".to_string(),
        ),
    };

    // The deeper reading, once streaming, takes the body over from the
    // offline weave (same no-spoiler gate).
    if settled && !deep.text.is_empty() {
        body = deep.text.clone();
    }
    let hint = if !deep.available {
        "No API key — run `taro-app --set-api-key` (or set ANTHROPIC_API_KEY) for a deeper reading"
            .to_string()
    } else {
        match deep.state {
            DeepState::Idle => "D — ask Claude for a deeper reading".to_string(),
            DeepState::Streaming if deep.text.is_empty() => {
                "Claude is contemplating the spread…".to_string()
            }
            DeepState::Streaming => "Claude is reading…".to_string(),
            DeepState::Done => "D — read again   ·   Tab — close".to_string(),
            DeepState::Error => format!("The deeper reading failed: {}", deep.error),
        }
    };

    if let Ok(mut t) = texts.p0().single_mut() {
        **t = heading;
    }
    if let Ok(mut t) = texts.p1().single_mut() {
        **t = body;
    }
    if let Ok(mut t) = texts.p2().single_mut() {
        **t = hint;
    }
}
