//! The reading panel (right side) and the title (top-left), both world-space so
//! they share the card rendering model. Text entities are tagged and rewritten
//! by [`update_panel`] whenever the selected card changes.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

use crate::cards::reveal_done_time;
use crate::theme::{GOLD, GOLD_DIM, MUTED, PARCHMENT};
use crate::{DealInfo, Fonts, ReadingData, Selected, Textures};

// Panel geometry (world space).
const PANEL_CX: f32 = 436.0;
const PANEL_CY: f32 = 6.0;
const PANEL_W: f32 = 360.0;
const PANEL_H: f32 = 612.0;
const PAD: f32 = 24.0;
const TEXT_X: f32 = PANEL_CX - PANEL_W / 2.0 + PAD;
const TEXT_W: f32 = PANEL_W - 2.0 * PAD;
const Z_PANEL: f32 = 30.0;
const Z_TEXT: f32 = 31.0;

#[derive(Component)]
pub struct PanelPositionText;
#[derive(Component)]
pub struct PanelCardText;
#[derive(Component)]
pub struct PanelOrientationText;
#[derive(Component)]
pub struct PanelMeaningText;
#[derive(Component)]
pub struct PanelDescriptionText;

fn roman(n: usize) -> &'static str {
    ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"]
        .get(n)
        .copied()
        .unwrap_or("")
}

fn divider(commands: &mut Commands, y: f32, width: f32) {
    commands.spawn((
        Sprite::from_color(GOLD_DIM.with_alpha(0.5), Vec2::new(width, 1.5)),
        Transform::from_xyz(PANEL_CX, y, Z_TEXT),
    ));
}

/// Spawn the title block in the top-left corner.
pub fn spawn_title(commands: &mut Commands, fonts: &Fonts) {
    commands.spawn((
        Text2d::new("Taro"),
        TextFont { font: fonts.bold.clone(), font_size: 44.0, ..default() },
        TextColor(GOLD),
        Anchor::TOP_LEFT,
        Transform::from_xyz(-612.0, 400.0, Z_TEXT),
    ));
    commands.spawn((
        Text2d::new("Tarot de Marseille  ·  the Celtic Cross"),
        TextFont { font: fonts.italic.clone(), font_size: 17.0, ..default() },
        TextColor(MUTED),
        Anchor::TOP_LEFT,
        Transform::from_xyz(-610.0, 350.0, Z_TEXT),
    ));
}

/// Spawn the reading panel scaffold. Text is filled by [`update_panel`].
pub fn spawn_panel(commands: &mut Commands, textures: &Textures, fonts: &Fonts) {
    // Background.
    commands.spawn((
        Sprite {
            image: textures.panel.clone(),
            custom_size: Some(Vec2::new(PANEL_W, PANEL_H)),
            ..default()
        },
        Transform::from_xyz(PANEL_CX, PANEL_CY, Z_PANEL),
    ));

    let top = PANEL_CY + PANEL_H / 2.0;

    // Section heading.
    commands.spawn((
        Text2d::new("THE READING"),
        TextFont { font: fonts.bold.clone(), font_size: 16.0, ..default() },
        TextColor(GOLD_DIM),
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, top - 26.0, Z_TEXT),
    ));
    divider(commands, top - 52.0, TEXT_W);

    // Position ("I · The Present").
    commands.spawn((
        Text2d::new(""),
        TextFont { font: fonts.regular.clone(), font_size: 18.0, ..default() },
        TextColor(GOLD),
        TextBounds { width: Some(TEXT_W), height: None },
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, top - 68.0, Z_TEXT),
        PanelPositionText,
    ));

    // Card name.
    commands.spawn((
        Text2d::new(""),
        TextFont { font: fonts.bold.clone(), font_size: 27.0, ..default() },
        TextColor(PARCHMENT),
        TextBounds { width: Some(TEXT_W), height: None },
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, top - 100.0, Z_TEXT),
        PanelCardText,
    ));

    // Orientation.
    commands.spawn((
        Text2d::new(""),
        TextFont { font: fonts.italic.clone(), font_size: 16.0, ..default() },
        TextColor(MUTED),
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, top - 140.0, Z_TEXT),
        PanelOrientationText,
    ));

    divider(commands, top - 162.0, TEXT_W);

    // Meaning body (wraps within the panel).
    commands.spawn((
        Text2d::new(""),
        TextFont { font: fonts.regular.clone(), font_size: 18.0, ..default() },
        TextColor(PARCHMENT),
        TextLayout::new_with_justify(Justify::Left),
        TextBounds { width: Some(TEXT_W), height: Some(360.0) },
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, top - 182.0, Z_TEXT),
        PanelMeaningText,
    ));

    // Lower block: what this position means in the spread.
    divider(commands, PANEL_CY - PANEL_H / 2.0 + 158.0, TEXT_W);
    commands.spawn((
        Text2d::new("THIS POSITION"),
        TextFont { font: fonts.bold.clone(), font_size: 13.0, ..default() },
        TextColor(GOLD_DIM),
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, PANEL_CY - PANEL_H / 2.0 + 142.0, Z_TEXT),
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont { font: fonts.italic.clone(), font_size: 16.0, ..default() },
        TextColor(MUTED),
        TextLayout::new_with_justify(Justify::Left),
        TextBounds { width: Some(TEXT_W), height: Some(100.0) },
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, PANEL_CY - PANEL_H / 2.0 + 118.0, Z_TEXT),
        PanelDescriptionText,
    ));

    // Footer hint.
    commands.spawn((
        Text2d::new("Click a card to read it\nSpace · new reading    R · reduced motion"),
        TextFont { font: fonts.italic.clone(), font_size: 14.0, ..default() },
        TextColor(GOLD_DIM),
        TextLayout::new_with_justify(Justify::Left),
        Anchor::TOP_LEFT,
        Transform::from_xyz(TEXT_X, PANEL_CY - PANEL_H / 2.0 + 52.0, Z_TEXT),
    ));
}

/// Rewrite the panel text when the selection changes or its card is revealed.
/// Until a card's flip completes, its reading is hidden (no spoilers).
#[allow(clippy::type_complexity)]
pub fn update_panel(
    selected: Res<Selected>,
    reading: Res<ReadingData>,
    deal_info: Option<Res<DealInfo>>,
    time: Res<Time>,
    mut last: Local<Option<(usize, bool)>>,
    mut sets: ParamSet<(
        Query<&mut Text2d, With<PanelPositionText>>,
        Query<&mut Text2d, With<PanelCardText>>,
        Query<(&mut Text2d, &mut TextColor), With<PanelOrientationText>>,
        Query<&mut Text2d, With<PanelMeaningText>>,
        Query<&mut Text2d, With<PanelDescriptionText>>,
    )>,
) {
    let revealed = match &deal_info {
        Some(info) => info.reduced || time.elapsed_secs() >= info.t0 + reveal_done_time(selected.0),
        None => false,
    };
    let key = (selected.0, revealed);
    if *last == Some(key) {
        return;
    }
    *last = Some(key);

    let entry = reading.0.entries.get(selected.0);
    let (pos, name, orient, meaning, desc) = match (revealed, entry) {
        (true, Some(e)) => (
            format!("{} · {}", roman(e.position_index), e.position_name),
            e.card_name.clone(),
            e.drawn.orientation,
            e.meaning.clone(),
            e.position_description.clone(),
        ),
        _ => (
            String::new(),
            String::new(),
            taro_domain::Orientation::Upright,
            String::new(),
            String::new(),
        ),
    };

    if let Ok(mut t) = sets.p0().single_mut() {
        **t = pos;
    }
    if let Ok(mut t) = sets.p1().single_mut() {
        **t = name;
    }
    if let Ok((mut t, mut color)) = sets.p2().single_mut() {
        let reversed = revealed && orient == taro_domain::Orientation::Reversed;
        **t = if !revealed {
            String::new()
        } else if reversed {
            "Reversed".into()
        } else {
            "Upright".into()
        };
        color.0 = if reversed { Color::srgb(0.78, 0.52, 0.46) } else { MUTED };
    }
    if let Ok(mut t) = sets.p3().single_mut() {
        **t = meaning;
    }
    if let Ok(mut t) = sets.p4().single_mut() {
        **t = desc;
    }
}
