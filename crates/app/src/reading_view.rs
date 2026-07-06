//! The full-reading overlay: a toggled panel that weaves the whole spread into
//! one flowing narrative (`taro_domain::compose_reading`), and hosts the Claude
//! "deeper reading" (Phase 5) once requested.
//!
//! `Tab` toggles it; the env `TARO_SHOW_READING_AT=secs` opens it once for the
//! headless capture harness. Honouring the panel's no-spoiler rule, the prose is
//! withheld until every card has flipped up.
//!
//! Unlike the rest of the scene (world-space sprites), the overlay is bevy_ui:
//! the body sits in an `Overflow::scroll_y()` container so readings longer than
//! the panel scroll (mouse wheel, ↑/↓, PageUp/PageDown, Home/End).

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use taro_domain::compose_reading;

use crate::ai::{DeepReading, DeepState};
use crate::cards::reveal_done_time;
use crate::question::QuestionInput;
use crate::theme::{Theme, Themed};
use crate::{DealInfo, Fonts, ReadingData, Textures};

/// The last-dealt card (index 9) flips last, so its reveal time gates the whole.
const LAST_CARD: usize = 9;

/// Scroll steps in logical pixels.
const LINE_STEP: f32 = 28.0;
const PAGE_STEP: f32 = 480.0;

/// Whether every card of the current deal has flipped face-up (the no-spoiler
/// gate shared by the overlay and the deeper-reading trigger).
pub fn spread_settled(deal_info: Option<&DealInfo>, now: f32) -> bool {
    deal_info.is_some_and(|info| info.reduced || now >= info.t0 + reveal_done_time(LAST_CARD))
}

/// Whether the full-reading overlay is showing.
#[derive(Resource, Default)]
pub struct ShowFullReading(pub bool);

/// The full-screen root node; its background doubles as the dimming scrim.
#[derive(Component)]
pub struct FullReadingRoot;
#[derive(Component)]
pub struct FullReadingHeading;
#[derive(Component)]
pub struct FullReadingBody;
#[derive(Component)]
pub struct FullReadingHint;
/// The `Overflow::scroll_y()` container around the body text.
#[derive(Component)]
pub struct FullReadingScroll;

/// Spawn the overlay (hidden). Toggled by [`update_full_reading`] via the root's
/// `Visibility`, which the children inherit.
pub fn spawn_full_reading(
    commands: &mut Commands,
    textures: &Textures,
    fonts: &Fonts,
    theme: &Theme,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme.clear().with_alpha(0.92)),
            Visibility::Hidden,
            FullReadingRoot,
        ))
        .with_children(|p| {
            // Framed panel (same procedural texture as the per-card panel).
            p.spawn((
                ImageNode::new(textures.panel.clone()).with_mode(bevy::ui::widget::NodeImageMode::Stretch),
                Node {
                    width: Val::Px(980.0),
                    height: Val::Px(720.0),
                    max_width: Val::Percent(96.0),
                    max_height: Val::Percent(94.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(50.0), Val::Px(28.0)),
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            ))
            .with_children(|p| {
                // Heading (the question, or a generic title).
                p.spawn((
                    Text::new(String::new()),
                    TextFont { font: fonts.bold.clone(), font_size: 24.0, ..default() },
                    TextColor(theme.gold()),
                    Themed::Gold,
                    TextLayout::new_with_justify(Justify::Center),
                    FullReadingHeading,
                ));
                // Scrollable body (woven prose / streamed deeper reading).
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        overflow: Overflow::scroll_y(),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ScrollPosition(Vec2::ZERO),
                    FullReadingScroll,
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new(String::new()),
                        TextFont { font: fonts.regular.clone(), font_size: 16.0, ..default() },
                        TextColor(theme.parchment()),
                        Themed::Parchment,
                        TextLayout::new_with_justify(Justify::Left),
                        FullReadingBody,
                    ));
                });
                // Hint / status line for the Claude deeper reading (Phase 5).
                p.spawn((
                    Text::new(String::new()),
                    TextFont { font: fonts.italic.clone(), font_size: 14.0, ..default() },
                    TextColor(theme.gold_dim()),
                    Themed::GoldDim,
                    TextLayout::new_with_justify(Justify::Center),
                    FullReadingHint,
                ));
            });
        });
}

/// `Tab` toggles the overlay (suppressed while typing a question); the env
/// `TARO_SHOW_READING_AT=secs` fires one open for the harness. Opening resets
/// the scroll to the top.
pub fn toggle_full_reading(
    keys: Res<ButtonInput<KeyCode>>,
    q: Res<QuestionInput>,
    time: Res<Time>,
    mut at: Local<Option<f32>>,
    mut init: Local<bool>,
    mut show: ResMut<ShowFullReading>,
    mut scroll: Query<&mut ScrollPosition, With<FullReadingScroll>>,
) {
    if !*init {
        *init = true;
        *at = std::env::var("TARO_SHOW_READING_AT").ok().and_then(|s| s.parse().ok());
    }
    let env_fire = at.is_some_and(|t| time.elapsed_secs() >= t);
    if env_fire {
        *at = None;
    } else if q.editing || !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    show.0 = env_fire || !show.0;
    if show.0 {
        if let Ok(mut sp) = scroll.single_mut() {
            sp.0 = Vec2::ZERO;
        }
    }
}

/// Mouse wheel and keys scroll the reading while the overlay is open. The
/// current position is re-derived from the clamped `ComputedNode` value each
/// time, so the `ScrollPosition` component can't drift past the content.
pub fn scroll_full_reading(
    show: Res<ShowFullReading>,
    q: Res<QuestionInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut env_scroll: Local<Option<f32>>,
    mut init: Local<bool>,
    mut scroll: Query<(&mut ScrollPosition, &ComputedNode), With<FullReadingScroll>>,
) {
    if !*init {
        *init = true;
        // Harness hook: apply one absolute scroll once the overlay is open,
        // since the headless capture can't inject wheel/key input.
        *env_scroll = std::env::var("TARO_SCROLL").ok().and_then(|s| s.parse().ok());
    }
    if !show.0 || q.editing {
        wheel.clear();
        return;
    }
    let Ok((mut sp, node)) = scroll.single_mut() else { return };
    if let Some(y) = env_scroll.take() {
        sp.0.y = y;
        return;
    }

    let mut dy = 0.0;
    for ev in wheel.read() {
        dy -= match ev.unit {
            MouseScrollUnit::Line => ev.y * LINE_STEP,
            MouseScrollUnit::Pixel => ev.y,
        };
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        dy += LINE_STEP;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        dy -= LINE_STEP;
    }
    if keys.just_pressed(KeyCode::PageDown) {
        dy += PAGE_STEP;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        dy -= PAGE_STEP;
    }
    if keys.just_pressed(KeyCode::End) {
        dy += f32::MAX / 2.0;
    }
    let home = keys.just_pressed(KeyCode::Home);
    if dy != 0.0 || home {
        // The layout system clamps into ComputedNode (physical px); start from
        // that instead of our own last value.
        let current = node.scroll_position.y * node.inverse_scale_factor();
        sp.0.y = if home { 0.0 } else { (current + dy).max(0.0) };
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
    mut root: Query<(&mut Visibility, &mut BackgroundColor), With<FullReadingRoot>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<FullReadingHeading>>,
        Query<&mut Text, With<FullReadingBody>>,
        Query<&mut Text, With<FullReadingHint>>,
    )>,
) {
    if let Ok((mut vis, mut scrim)) = root.single_mut() {
        *vis = if show.0 { Visibility::Visible } else { Visibility::Hidden };
        scrim.0 = theme.clear().with_alpha(0.92);
    }
    if !show.0 {
        return;
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
            DeepState::Done => "D — read again   ·   ↑↓ — scroll   ·   Tab — close".to_string(),
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
