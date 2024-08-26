use bevy::{
    color::palettes::tailwind::{self},
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
};
use bevy_egui::EguiPlugin;
use bevy_vello::{vello::kurbo, VelloPlugin, VelloScene, VelloSceneBundle};
use camera::{camera_controls, CameraSettings, ParticleCamera};
use compute::ComputePlugin;
use data::{AttractionRules, Particles, Shape};
use rayon::prelude::*;
use vello_utils::{SceneExt, ToPoint};

mod camera;
mod compute;
mod data;
mod draw;
mod spatial;
mod ui;
mod vello_utils;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin,
            VelloPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            ComputePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, update)
        // .add_systems(Update, (draw, ui::ui, camera_controls))
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

    let rules = AttractionRules::new_random(vec![
        tailwind::EMERALD_600.into(),
        tailwind::TEAL_600.into(),
        tailwind::BLUE_600.into(),
        tailwind::PURPLE_600.into(),
        tailwind::PINK_600.into(),
    ]);

    let mut particles = Particles::new();
    particles.change_particle_count(800, rules.min_max_x, rules.min_max_y, rules.colors.len());

    commands.insert_resource(particles);
    commands.insert_resource(rules);
    commands.spawn(VelloSceneBundle::default());
}

fn update(
    time: Res<Time<Virtual>>,
    rules: Res<AttractionRules>,
    mut particles_write: ResMut<Particles>,
    mut particles_read: Local<Particles>,
) {
    let relative_min_distance = rules.min_distance / rules.max_distance;
    particles_read.clear();
    particles_read.extend(particles_write.iter().copied());

    particles_write.par_iter_mut().for_each(|particle| {
        particle.velocity *= 0.5f32.powf(time.delta_seconds() / rules.velocity_half_life);

        for other_particle in particles_read.iter() {
            let other_position =
                rules.closest_wrapped_other_position(particle.position, other_particle.position);

            let distance_squared = particle.position.distance_squared(other_position);
            if distance_squared == 0. || distance_squared > rules.max_distance * rules.max_distance
            {
                continue;
            }

            let attraction = rules.get_attraction(particle.color, other_particle.color);
            let relative_position = other_position - particle.position;

            let acceleration = (rules.acceleration_fn)(
                relative_min_distance,
                relative_position / rules.max_distance,
                attraction,
            );
            particle.velocity +=
                acceleration * rules.max_distance * rules.force_factor * time.delta_seconds();
        }

        let velocity = particle.velocity.clamp_length_max(rules.max_velocity);
        particle.position += velocity * time.delta_seconds();

        if particle.position.x > rules.min_max_x {
            particle.position.x -= 2. * rules.min_max_x;
        }
        if particle.position.x < -rules.min_max_x {
            particle.position.x += 2. * rules.min_max_x;
        }
        if particle.position.y > rules.min_max_y {
            particle.position.y -= 2. * rules.min_max_y;
        }
        if particle.position.y < -rules.min_max_y {
            particle.position.y += 2. * rules.min_max_y;
        }
    });
}

fn draw(
    mut scene: Query<&mut VelloScene>,
    particles: ResMut<Particles>,
    rules: Res<AttractionRules>,
) {
    let mut scene = scene.single_mut();
    *scene = VelloScene::new();

    for particle in particles.iter() {
        let c = rules.get_color(particle.color).to_srgba().to_u8_array();
        // println!("{:?}", c);
        let drawing = scene
            .builder()
            .with_default_fill()
            .with_fill_color(c.into());
        match particles.shape {
            Shape::Circle => {
                drawing.draw_circle(particle.position.into(), particles.particle_size as f64)
            }
            Shape::Square => drawing.draw(kurbo::Rect::from_center_size(
                particle.position.to_point(),
                (
                    particles.particle_size as f64,
                    particles.particle_size as f64,
                ),
            )),
        }

        //
    }
}
