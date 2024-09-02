use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::extract_component::ExtractComponent,
};
use bevy_egui::EguiContexts;

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
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mut egui_contexts: EguiContexts,
) {
    let egui_context = egui_contexts.ctx_mut();
    let block_mouse = egui_context.is_pointer_over_area() || egui_context.is_using_pointer();

    let mut translation_from_mouse = Vec3::ZERO;
    if mouse.pressed(MouseButton::Left) | mouse.pressed(MouseButton::Right) {
        for event in mouse_motion.read() {
            translation_from_mouse += Vec3::new(-event.delta.x, event.delta.y, 0.);
        }
    } else {
        mouse_motion.read();
    }

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
    camera.translation += Vec3::new(translation.x, translation.y, 0.) * 2. * settings.pan_speed;

    if !block_mouse {
        for event in mouse_wheel.read() {
            projection.scale *= 1. - settings.scroll_speed * event.y / 20.;
        }

        camera.translation += translation_from_mouse * projection.scale;
    }
}
