//! The querent's question: an in-app text field (top banner) plus a
//! `TARO_QUESTION` env seed so the headless capture harness can drive it.
//!
//! Typing is modal — `Enter` focuses the field, and while focused the redeal and
//! theme hotkeys are suppressed (see the `editing` gate in `interact::redeal_input`
//! and `main::cycle_theme_input`) so spaces and letters land in the text instead.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

use crate::theme::Theme;
use crate::Fonts;

/// Cap on the question length (bytes), guarding the banner width.
const MAX_LEN: usize = 120;
const Z_TEXT: f32 = 31.0;

/// The querent's question and whether the field is being edited.
#[derive(Resource, Default)]
pub struct QuestionInput {
    pub text: String,
    pub editing: bool,
}

#[derive(Component)]
pub struct QuestionBanner;

/// Spawn the question banner across the top, above the spread. Its colour and
/// text are driven each frame by [`update_question_text`].
pub fn spawn_question(commands: &mut Commands, fonts: &Fonts, theme: &Theme) {
    commands.spawn((
        Text2d::new(String::new()),
        TextFont { font: fonts.regular.clone(), font_size: 19.0, ..default() },
        TextColor(theme.muted()),
        TextLayout::new_with_justify(Justify::Center),
        TextBounds { width: Some(840.0), height: None },
        Anchor::TOP_CENTER,
        Transform::from_xyz(-140.0, 412.0, Z_TEXT),
        QuestionBanner,
    ));
}

/// Capture keystrokes into the question while editing; `Enter` focuses the
/// field, `Enter`/`Esc` commit, `Backspace` deletes.
pub fn question_input(mut events: MessageReader<KeyboardInput>, mut q: ResMut<QuestionInput>) {
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        if !q.editing {
            if matches!(ev.logical_key, Key::Enter) {
                q.editing = true;
            }
            continue;
        }
        match &ev.logical_key {
            Key::Enter | Key::Escape => q.editing = false,
            Key::Backspace => {
                q.text.pop();
            }
            Key::Space => {
                if q.text.len() < MAX_LEN {
                    q.text.push(' ');
                }
            }
            Key::Character(s) => {
                for c in s.chars() {
                    if q.text.len() < MAX_LEN && !c.is_control() {
                        q.text.push(c);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Rewrite the banner: an italic-muted placeholder when empty and idle, the
/// question (parchment) otherwise, with a blinking caret while editing.
pub fn update_question_text(
    q: Res<QuestionInput>,
    time: Res<Time>,
    theme: Res<Theme>,
    mut banner: Query<(&mut Text2d, &mut TextColor), With<QuestionBanner>>,
) {
    let Ok((mut text, mut color)) = banner.single_mut() else {
        return;
    };
    if q.text.is_empty() && !q.editing {
        **text = "Press Enter to ask a question…".into();
        color.0 = theme.muted();
    } else {
        let caret = if q.editing && (time.elapsed_secs() * 1.6).fract() < 0.5 { "|" } else { "" };
        **text = format!("“{}{caret}”", q.text);
        color.0 = theme.parchment();
    }
}
