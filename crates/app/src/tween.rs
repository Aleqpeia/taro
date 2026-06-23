//! A small hand-rolled tween system (bevy_tweening trails two Bevy versions).
//!
//! Tweens are keyed to `Time::elapsed_secs()` — accumulated real time — so the
//! choppy frame-rate under software rendering changes only smoothness, never a
//! tween's duration or final resting state.

use bevy::prelude::*;
use std::f32::consts::PI;

/// Easing curves, all mapping a linear `t` in `[0,1]` to an eased value.
#[derive(Clone, Copy)]
// OutBack/OutQuad round out the easing palette for future motion.
#[allow(dead_code, clippy::enum_variant_names)]
pub enum Ease {
    OutCubic,
    OutBack,
    OutQuad,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::OutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::OutQuad => 1.0 - (1.0 - t).powi(2),
            Ease::OutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
        }
    }
}

/// Animates a `Transform` from `start` to `end`, with an optional ballistic
/// arc (a vertical lift peaking at the midpoint — the "deal" toss).
#[derive(Component)]
pub struct TransformTween {
    pub start: Transform,
    pub end: Transform,
    pub start_time: f32,
    pub duration: f32,
    pub ease: Ease,
    pub arc: f32,
    pub done: bool,
}

impl TransformTween {
    pub fn new(start: Transform, end: Transform, start_time: f32, duration: f32, ease: Ease) -> Self {
        Self { start, end, start_time, duration, ease, arc: 0.0, done: false }
    }

    pub fn with_arc(mut self, arc: f32) -> Self {
        self.arc = arc;
        self
    }
}

/// A two-stage flip: shrink to zero width showing the back, swap to the face at
/// the midpoint, then grow back to full width.
#[derive(Component)]
pub struct FlipReveal {
    pub start_time: f32,
    /// Duration of each half (back-out, face-in).
    pub half: f32,
    pub face: Handle<Image>,
    pub swapped: bool,
    pub done: bool,
}

/// Fades a sprite or text from transparent to opaque over a window.
#[derive(Component)]
pub struct FadeIn {
    pub start_time: f32,
    pub duration: f32,
}

/// Plugin wiring all tween driver systems.
pub struct TweenPlugin;

impl Plugin for TweenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (drive_transform_tweens, drive_flips, fade_in_sprites, fade_in_texts),
        );
    }
}

fn drive_transform_tweens(time: Res<Time>, mut q: Query<(&mut Transform, &mut TransformTween)>) {
    let now = time.elapsed_secs();
    for (mut tf, mut tw) in &mut q {
        if tw.done {
            continue;
        }
        if now < tw.start_time {
            *tf = tw.start; // wait in the deck
            continue;
        }
        let raw = ((now - tw.start_time) / tw.duration).clamp(0.0, 1.0);
        let e = tw.ease.apply(raw);
        let mut pos = tw.start.translation.lerp(tw.end.translation, e);
        pos.y += (PI * raw).sin() * tw.arc;
        tf.translation = pos;
        tf.rotation = tw.start.rotation.slerp(tw.end.rotation, e);
        tf.scale = tw.start.scale.lerp(tw.end.scale, e);
        if raw >= 1.0 {
            *tf = tw.end;
            tw.done = true;
        }
    }
}

fn drive_flips(time: Res<Time>, mut q: Query<(&mut Transform, &mut Sprite, &mut FlipReveal)>) {
    let now = time.elapsed_secs();
    for (mut tf, mut sprite, mut fr) in &mut q {
        if fr.done || now < fr.start_time {
            continue;
        }
        let phase = ((now - fr.start_time) / fr.half).min(2.0); // 0..2
        tf.scale.x = (1.0 - phase).abs().max(0.0001);
        if phase >= 1.0 && !fr.swapped {
            sprite.image = fr.face.clone();
            fr.swapped = true;
        }
        if phase >= 2.0 {
            tf.scale.x = 1.0;
            fr.done = true;
        }
    }
}

fn ramp(now: f32, start: f32, dur: f32) -> f32 {
    ((now - start) / dur).clamp(0.0, 1.0)
}

fn fade_in_sprites(time: Res<Time>, mut q: Query<(&FadeIn, &mut Sprite)>) {
    let now = time.elapsed_secs();
    for (f, mut sprite) in &mut q {
        sprite.color.set_alpha(ramp(now, f.start_time, f.duration));
    }
}

fn fade_in_texts(time: Res<Time>, mut q: Query<(&FadeIn, &mut TextColor)>) {
    let now = time.elapsed_secs();
    for (f, mut color) in &mut q {
        color.0.set_alpha(ramp(now, f.start_time, f.duration));
    }
}
