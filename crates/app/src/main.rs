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
mod textures;
mod theme;
mod tween;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use rand::thread_rng;

use cards::SpreadEntity;
use taro_domain::{build_reading, CelticCross, Deck, LayoutSlot, Meanings, Reading, Spread};
use theme::{WIN_H, WIN_W};

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
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.024, 0.052)))
        .insert_resource(Selected(select0))
        .insert_resource(Motion { reduced: reduced_env })
        .insert_resource(DebugRedeal { at: redeal_at, done: false })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Taro — Tarot de Marseille".into(),
                resolution: WindowResolution::new(WIN_W as u32, WIN_H as u32),
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
            ),
        )
        .run();
}

fn setup_assets(mut commands: Commands, mut images: ResMut<Assets<Image>>, assets: Res<AssetServer>) {
    commands.insert_resource(Textures {
        vignette: images.add(textures::vignette_image(WIN_W as usize, WIN_H as usize)),
        shadow: images.add(textures::soft_shadow_image(256)),
        card_back: images.add(textures::card_back_image(192, 360)),
        disc: images.add(textures::disc_image(52)),
        panel: images.add(textures::panel_image(360, 612)),
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
) {
    commands.spawn(Camera2d);

    // Felt background.
    commands.spawn((
        Sprite {
            image: textures.vignette.clone(),
            custom_size: Some(Vec2::new(WIN_W, WIN_H)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -100.0),
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

    panel::spawn_title(&mut commands, &fonts);
    panel::spawn_panel(&mut commands, &textures, &fonts);

    deal(&mut commands, &textures, &fonts, &assets, motion.reduced, time.elapsed_secs());
}

/// Shuffle, deal a Celtic Cross, and spawn the cards. Reused for redeals.
pub(crate) fn deal(
    commands: &mut Commands,
    textures: &Textures,
    fonts: &Fonts,
    assets: &AssetServer,
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
