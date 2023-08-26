use bevy::core::{Pod, Zeroable};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::ggrs::PlayerHandle;

use crate::MainCamera;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Pod, Zeroable, Debug, Default, Reflect)]
pub struct GaffInput {
    pub mouse_pos: Vec2,
    pub buttons: u8,
    _padding: [u8; 3],
}

pub const INPUT_UP: u8 = 1 << 0;
pub const INPUT_DOWN: u8 = 1 << 1;
pub const INPUT_LEFT: u8 = 1 << 2;
pub const INPUT_RIGHT: u8 = 1 << 3;
pub const INPUT_MOUSE_LEFT: u8 = 1 << 4;

pub fn input(
    _handle: In<PlayerHandle>,
    keyboard: Res<Input<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mouse_buttons: Res<Input<MouseButton>>,
) -> GaffInput {
    let mut input: u8 = 0;

    if keyboard.pressed(KeyCode::W) {
        input |= INPUT_UP;
    }
    if keyboard.pressed(KeyCode::A) {
        input |= INPUT_LEFT;
    }
    if keyboard.pressed(KeyCode::S) {
        input |= INPUT_DOWN;
    }
    if keyboard.pressed(KeyCode::D) {
        input |= INPUT_RIGHT;
    }

    if mouse_buttons.pressed(MouseButton::Left) {
        input |= INPUT_MOUSE_LEFT;
    }

    let (camera, camera_transform) = cameras.single();
    let mouse_pos = windows
        .single()
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
        .unwrap_or(Vec2::ZERO);

    GaffInput {
        buttons: input,
        mouse_pos,
        ..default()
    }
}
