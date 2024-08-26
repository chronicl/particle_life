use bevy::{input::mouse::MouseWheel, prelude::*, render::extract_component::ExtractComponent};
use rand::seq::SliceRandom;

use crate::data::COLORS;

#[derive(Component, ExtractComponent, Debug, Clone, Copy, Default)]
pub struct ParticleCamera;

#[derive(Component, Debug, Clone, Copy)]
pub struct CameraSettings {
    pub pan_speed: f32,
    pub scroll_speed: f32,
}

pub fn camera_controls(
    mut camera: Query<(&mut Transform, &mut OrthographicProjection, &CameraSettings)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,
) {
    let mut translation = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        translation.y += 1.;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        translation.y -= 1.;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        translation.x -= 1.;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        translation.x += 1.;
    }

    let (mut camera, mut projection, settings) = camera.single_mut();
    for event in mouse_wheel.read() {
        projection.scale *= 1. - settings.scroll_speed * event.y / 20.;
    }

    camera.translation += Vec3::new(translation.x, translation.y, 0.) * 2. * settings.pan_speed;
}
