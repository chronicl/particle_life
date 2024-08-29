use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_egui::EguiPlugin;

use camera::{camera_controls, CameraSettings, ParticleCamera};
use compute::ComputePlugin;
use data::SimulationSettings;
use draw::DrawPlugin;
use events::ParticleEvent;

mod camera;
mod compute;
mod data;
mod draw;
mod events;
mod ui;

fn main() {
    App::new()
        .add_event::<ParticleEvent>()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin,
            // Used by ui to display the fps.
            FrameTimeDiagnosticsPlugin::default(),
            ComputePlugin,
            DrawPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (ui::ui, camera_controls))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle::default(),
        CameraSettings {
            pan_speed: 1.,
            scroll_speed: 1.,
        },
        ParticleCamera,
    ));

    let mut settings = SimulationSettings::default();
    settings.randomize_attractions();
    commands.insert_resource(settings);
}
