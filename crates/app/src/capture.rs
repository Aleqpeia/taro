//! Deterministic screenshot harness — our "eyes" for verifying the visuals.
//!
//! Because this is a graphical app, correctness ("is it aligned, visible,
//! good-looking?") cannot be judged from logs. This plugin lets a headless
//! background run capture the window at a fixed wall-clock time and then exit,
//! so a screenshot can be read back and inspected.
//!
//! Activated by environment variables (no-op when unset, so normal runs are
//! unaffected):
//!
//! * `TARO_CAPTURE` — output PNG path. Presence enables capture mode.
//! * `TARO_CAPTURE_AT` — seconds of wall-clock to wait before capturing
//!   (default `3.0`). Tweens are keyed to elapsed time, so this deterministically
//!   selects an animation frame.
//!
//! After the shot is requested we wait a short grace period (the save is async,
//! handed to the render world over a channel) and then send `AppExit`.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

/// Grace period between requesting the screenshot and exiting, to let the
/// render world receive the captured image and flush it to disk.
const SAVE_GRACE: Duration = Duration::from_millis(1500);

#[derive(Resource)]
struct CaptureConfig {
    path: String,
    at: Duration,
    /// Wall-clock start, so capture timing is independent of frame rate.
    started: Instant,
    requested: bool,
    request_time: Option<Instant>,
}

/// Adds deterministic capture-and-exit behaviour when `TARO_CAPTURE` is set.
pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        let Ok(path) = std::env::var("TARO_CAPTURE") else {
            return;
        };
        let at = std::env::var("TARO_CAPTURE_AT")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(3.0)
            .max(0.0);

        app.insert_resource(CaptureConfig {
            path,
            at: Duration::from_secs_f32(at),
            started: Instant::now(),
            requested: false,
            request_time: None,
        })
        .add_systems(Update, drive_capture);
    }
}

fn drive_capture(
    mut commands: Commands,
    mut cfg: ResMut<CaptureConfig>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = cfg.started.elapsed();

    if !cfg.requested {
        if elapsed >= cfg.at {
            let path = cfg.path.clone();
            info!("capture: requesting screenshot -> {path}");
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            cfg.requested = true;
            cfg.request_time = Some(Instant::now());
        }
        return;
    }

    if let Some(t) = cfg.request_time {
        if t.elapsed() >= SAVE_GRACE {
            info!("capture: done, exiting");
            exit.write(AppExit::Success);
        }
    }
}
