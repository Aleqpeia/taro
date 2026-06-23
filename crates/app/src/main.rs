//! Taro — Phase 3: an animated, themed Celtic Cross reading.
//!
//! Built on the engine-agnostic `taro-domain` core. This binary owns all
//! presentation: a themed felt table, procedurally generated card backs and
//! shadows, a hand-rolled tween system (bevy_tweening lags two Bevy versions),
//! the shuffle → deal → reveal animation, and a reading panel.

mod capture;
mod cards;
mod interact;
mod layout;
mod panel;
mod question;
mod reading_view;
mod textures;
mod theme;
mod tween;

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use rand::thread_rng;

use cards::SpreadEntity;
use question::QuestionInput;
use reading_view::ShowFullReading;
use taro_domain::{build_reading, CelticCross, Deck, LayoutSlot, Meanings, Reading, Spread};
use theme::{theme_index_by_name, Theme, ThemeIndex, Themed, WIN_H, WIN_W, THEMES};

/// Handles to the procedurally generated textures, created once at startup.
#[derive(Resource)]
struct Textures {
    vignette: Handle<Image>,
    shadow: Handle<Image>,
    card_back: Handle<Image>,
    disc: Handle<Image>,
    panel: Handle<Image>,
    glow: Handle<Image>,
}

/// Loaded UI fonts.
#[derive(Resource)]
struct Fonts {
    regular: Handle<Font>,
    bold: Handle<Font>,
    italic: Handle<Font>,
}

/// The current reading and the layout slot each entry was dealt to.
#[derive(Resource)]
struct ReadingData(Reading);

/// Which entry the reading panel is showing.
#[derive(Resource)]
struct Selected(usize);

/// Timing/mode of the active deal, for animation gating and redeals.
#[derive(Resource)]
struct DealInfo {
    t0: f32,
    reduced: bool,
}

/// User preference for reduced motion, applied to the next deal.
#[derive(Resource)]
struct Motion {
    reduced: bool,
}

/// Debug hook (env `TARO_REDEAL_AT`): fire one redeal at the given elapsed time,
/// so redeals can be exercised by the headless capture harness.
#[derive(Resource)]
struct DebugRedeal {
    at: Option<f32>,
    done: bool,
}

fn main() {
    let reduced_env = std::env::var("TARO_REDUCED_MOTION").is_ok();
    let select0 = std::env::var("TARO_SELECT").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let redeal_at = std::env::var("TARO_REDEAL_AT").ok().and_then(|s| s.parse().ok());
    // Theme: env name (e.g. TARO_THEME=emerald) -> preset index, default 0.
    let theme_idx = std::env::var("TARO_THEME")
        .ok()
        .and_then(|s| theme_index_by_name(&s))
        .unwrap_or(0);
    let theme = THEMES[theme_idx];
    // Optional initial window size override (env `TARO_WINDOW=1600x720`), so the
    // headless harness can verify the fit-to-window scaling at other aspects.
    let (win_w, win_h) = std::env::var("TARO_WINDOW")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once(['x', 'X'])?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        })
        .unwrap_or((WIN_W, WIN_H));
    // Seed the question (env `TARO_QUESTION`) and optionally start in edit mode
    // (`TARO_EDIT_QUESTION`), so the harness can screenshot both the field and
    // the blinking caret without live keystrokes.
    let question = QuestionInput {
        text: std::env::var("TARO_QUESTION").unwrap_or_default(),
        editing: std::env::var("TARO_EDIT_QUESTION").is_ok(),
    };

    App::new()
        .insert_resource(ClearColor(theme.clear()))
        .insert_resource(theme)
        .insert_resource(ThemeIndex(theme_idx))
        .insert_resource(Selected(select0))
        .insert_resource(Motion { reduced: reduced_env })
        .insert_resource(DebugRedeal { at: redeal_at, done: false })
        .insert_resource(question)
        .insert_resource(ShowFullReading::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Taro — Tarot de Marseille".into(),
                // The starting size; the window is freely resizable and the
                // scene scales to fit (see the fit-to-window camera below).
                resolution: WindowResolution::new(win_w as u32, win_h as u32),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((capture::CapturePlugin, tween::TweenPlugin))
        .add_systems(Startup, (setup_assets, setup_scene).chain())
        .add_systems(
            Update,
            (
                panel::update_panel,
                interact::redeal_input,
                interact::select_input,
                interact::apply_selection_highlight,
                interact::debug_redeal,
                question::question_input,
                question::update_question_text,
                reading_view::toggle_full_reading,
                reading_view::update_full_reading,
                cycle_theme_input,
                apply_theme,
                fit_background,
            ),
        )
        .run();
}

fn setup_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    theme: Res<Theme>,
) {
    commands.insert_resource(Textures {
        vignette: images.add(textures::vignette_image(
            WIN_W as usize,
            WIN_H as usize,
            theme.felt_center,
            theme.felt_edge,
        )),
        shadow: images.add(textures::soft_shadow_image(256)),
        card_back: images.add(textures::card_back_image(192, 360, theme.card_field, theme.gold)),
        disc: images.add(textures::disc_image(52, theme.gold, theme.card_field)),
        panel: images.add(textures::panel_image(360, 612, theme.panel_fill, theme.gold)),
        glow: images.add(textures::glow_image(256)),
    });
    commands.insert_resource(Fonts {
        regular: assets.load(theme::FONT_REGULAR),
        bold: assets.load(theme::FONT_BOLD),
        italic: assets.load(theme::FONT_ITALIC),
    });
}

fn setup_scene(
    mut commands: Commands,
    textures: Res<Textures>,
    fonts: Res<Fonts>,
    assets: Res<AssetServer>,
    motion: Res<Motion>,
    time: Res<Time>,
    theme: Res<Theme>,
) {
    // Fit-to-window camera: the design canvas (WIN_W×WIN_H) is always fully
    // visible and scales with the window; on a different aspect the view grows
    // along the longer axis (filled by the felt), never clipping the layout.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin { min_width: WIN_W, min_height: WIN_H },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Felt background; `fit_background` keeps it covering the visible area.
    commands.spawn((
        Sprite {
            image: textures.vignette.clone(),
            custom_size: Some(Vec2::new(WIN_W, WIN_H)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -100.0),
        Background,
    ));

    // Gold selection glow (positioned/faded by the highlight system).
    commands.spawn((
        Sprite {
            image: textures.glow.clone(),
            custom_size: Some(Vec2::new(layout::CARD_W * 1.85, layout::CARD_H * 1.5)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.0),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 2.0),
        interact::Highlight,
    ));

    panel::spawn_title(&mut commands, &fonts, &theme);
    panel::spawn_panel(&mut commands, &textures, &fonts, &theme);
    question::spawn_question(&mut commands, &fonts, &theme);
    reading_view::spawn_full_reading(&mut commands, &textures, &fonts, &theme);

    deal(&mut commands, &textures, &fonts, &assets, &theme, motion.reduced, time.elapsed_secs());
}

/// Shuffle, deal a Celtic Cross, and spawn the cards. Reused for redeals.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deal(
    commands: &mut Commands,
    textures: &Textures,
    fonts: &Fonts,
    assets: &AssetServer,
    theme: &Theme,
    reduced: bool,
    t0: f32,
) {
    let mut rng = thread_rng();
    let mut deck = Deck::full();
    deck.shuffle(&mut rng);
    let spread = CelticCross;
    let drawn = deck.draw(&mut rng, spread.card_count());
    let meanings = Meanings::embedded();
    let reading = build_reading(&spread, &drawn, &meanings);
    let slots: Vec<LayoutSlot> = spread.positions().iter().map(|p| p.slot).collect();

    for (i, (entry, slot)) in reading.entries.iter().zip(slots.iter()).enumerate() {
        let face = assets.load(entry.drawn.card.asset_path());
        cards::spawn_card(
            commands,
            textures,
            fonts,
            theme.gold(),
            face,
            *slot,
            entry.drawn.orientation,
            entry.position_index,
            i,
            reduced,
            t0,
        );
    }

    commands.insert_resource(ReadingData(reading));
    commands.insert_resource(DealInfo { t0, reduced });
}

/// Despawn every entity belonging to the current deal (for redeals).
pub(crate) fn clear_spread(commands: &mut Commands, q: &Query<Entity, With<SpreadEntity>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

/// The felt sprite, stretched to cover the whole viewport.
#[derive(Component)]
struct Background;

/// Keep the felt covering the visible world area as the window resizes. The
/// fit-to-window camera grows `projection.area` beyond the design canvas on a
/// mismatched aspect; this stretches the felt to fill it.
fn fit_background(
    proj: Query<&Projection, With<Camera2d>>,
    mut bg: Query<&mut Sprite, With<Background>>,
) {
    let Ok(projection) = proj.single() else { return };
    let Projection::Orthographic(o) = projection else { return };
    let size = o.area.size() * 1.01; // hair of overscan to avoid edge seams
    for mut s in &mut bg {
        if s.custom_size != Some(size) {
            s.custom_size = Some(size);
        }
    }
}

/// `T` cycles to the next theme; changing the `Theme` resource triggers
/// [`apply_theme`]. The env `TARO_CYCLE_THEME_AT=secs` fires one cycle for the
/// headless harness, so the runtime re-skin path can be screenshot-verified.
fn cycle_theme_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    question: Res<QuestionInput>,
    mut at: Local<Option<f32>>,
    mut init: Local<bool>,
    mut idx: ResMut<ThemeIndex>,
    mut theme: ResMut<Theme>,
) {
    if !*init {
        *init = true;
        *at = std::env::var("TARO_CYCLE_THEME_AT").ok().and_then(|s| s.parse().ok());
    }
    let env_fire = at.is_some_and(|t| time.elapsed_secs() >= t);
    if env_fire {
        *at = None;
    }
    // `T` must reach the question field, not cycle the theme, while typing.
    if env_fire || (!question.editing && keys.just_pressed(KeyCode::KeyT)) {
        idx.0 = (idx.0 + 1) % THEMES.len();
        *theme = THEMES[idx.0];
    }
}

/// Re-skin the scene when the `Theme` resource changes: regenerate the
/// procedural images in place (every sprite holding the handle updates for
/// free), reset the clear colour, and recolour all themed text and dividers.
/// Dynamic panel text is refreshed separately by `update_panel`.
fn apply_theme(
    theme: Res<Theme>,
    mut started: Local<bool>,
    textures: Option<Res<Textures>>,
    mut images: ResMut<Assets<Image>>,
    mut clear: ResMut<ClearColor>,
    mut texts: Query<(&Themed, &mut TextColor)>,
    mut sprites: Query<(&Themed, &mut Sprite)>,
) {
    if !theme.is_changed() {
        return;
    }
    // The startup insert counts as a change, but setup_assets already built the
    // textures for it — skip that one to avoid regenerating identical images.
    if !*started {
        *started = true;
        return;
    }
    let Some(textures) = textures else { return };

    // Overwrite each procedural image behind its existing handle, so every
    // sprite already referencing it re-skins for free. insert only errors on a
    // stale handle generation, which can't happen for our strong handles.
    let mut reskin = |handle, image| {
        let _ = images.insert(handle, image);
    };
    reskin(
        &textures.vignette,
        textures::vignette_image(WIN_W as usize, WIN_H as usize, theme.felt_center, theme.felt_edge),
    );
    reskin(&textures.card_back, textures::card_back_image(192, 360, theme.card_field, theme.gold));
    reskin(&textures.panel, textures::panel_image(360, 612, theme.panel_fill, theme.gold));
    reskin(&textures.disc, textures::disc_image(52, theme.gold, theme.card_field));

    clear.0 = theme.clear();
    for (role, mut color) in &mut texts {
        color.0 = role.color(&theme);
    }
    for (role, mut sprite) in &mut sprites {
        sprite.color = role.color(&theme);
    }
}
