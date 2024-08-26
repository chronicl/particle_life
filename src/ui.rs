use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{
    egui::{self, color_picker::show_color},
    EguiContexts,
};

use crate::{
    camera::CameraSettings,
    data::{random_colors, AccelerationMethod, AttractionRules, Particles, Shape, COLORS},
};

pub fn ui(
    mut contexts: EguiContexts,
    mut rules: ResMut<AttractionRules>,
    mut particles: ResMut<Particles>,
    mut camera_settings: Query<&mut CameraSettings>,
    diagnostic: Res<DiagnosticsStore>,
    mut time: ResMut<Time<Virtual>>,
) {
    egui::Window::new("Settings").show(contexts.ctx_mut(), |ui| {
        if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                ui.label(format!("FPS: {value:.2}"));
            }
        }

        let mut relative_speed = time.relative_speed();
        let before = relative_speed;
        ui.add(
            egui::Slider::new(&mut relative_speed, 0.1..=10.)
                .text("simulation speed")
                .clamp_to_range(false),
        );
        if relative_speed != before {
            time.set_relative_speed(relative_speed);
        }

        let mut particle_count = particles.len();
        ui.add(
            egui::Slider::new(&mut particle_count, 0..=3000)
                .text("particle count")
                .clamp_to_range(false),
        );
        if particle_count != particles.len() {
            particles.change_particle_count(
                particle_count,
                rules.min_max_x,
                rules.min_max_y,
                rules.colors.len(),
            );
        }

        ui.add(
            egui::Slider::new(&mut rules.min_distance, 0.0..=200.0)
                .text("min distance")
                .clamp_to_range(false),
        );
        ui.add(
            egui::Slider::new(&mut rules.max_distance, 100.0..=1000.0)
                .text("max distance")
                .clamp_to_range(false),
        );
        ui.add(
            egui::Slider::new(&mut rules.max_velocity, 1.0..=1000.0)
                .text("max velocity")
                .clamp_to_range(false),
        );
        ui.add(
            egui::Slider::new(&mut rules.velocity_half_life, 0.001..=2.0)
                .text("velocity half life"),
        );
        ui.add(
            egui::Slider::new(&mut rules.force_factor, 0.0..=100.0)
                .text("force scale")
                .clamp_to_range(false),
        );
        ui.add(
            egui::Slider::new(&mut rules.min_max_x, 100.0..=3000.0)
                .text("bounds x")
                .clamp_to_range(false),
        );
        ui.add(
            egui::Slider::new(&mut rules.min_max_y, 100.0..=3000.0)
                .text("bounds y")
                .clamp_to_range(false),
        );

        let mut color_count = rules.colors.len();
        let before = color_count;
        ui.add(
            egui::Slider::new(&mut color_count, 1..=COLORS.len())
                .text("color count")
                .clamp_to_range(false),
        );
        if color_count != before {
            rules.change_colors(random_colors(color_count));
            particles.randomize_colors(color_count);
        }

        if rules.colors.len() < 11 {
            ui.add_space(10.);

            let color_ui = |ui: &mut egui::Ui, color: &Color| {
                let color = color.to_linear();
                show_color(
                    ui,
                    egui::Rgba::from_rgb(color.red, color.green, color.blue),
                    egui::Vec2::new(20., 20.),
                );
            };

            ui.horizontal(|ui| {
                color_ui(ui, &LinearRgba::NONE.into());
                for color in rules.colors.iter() {
                    color_ui(ui, color);
                }
            });
            for i in 0..rules.colors.len() {
                ui.horizontal(|ui| {
                    color_ui(ui, &rules.colors[i]);

                    for j in 0..rules.colors.len() {
                        ui.add(egui::DragValue::new(&mut rules.matrix[i][j]).speed(0.01));
                    }
                });
            }
        }

        if ui.button("Randomize attractions").clicked() {
            rules.randomize_attractions();
        }

        if ui.button("Randomize positions").clicked() {
            particles.randomize_positions(rules.min_max_x, rules.min_max_y);
        }

        if ui.button("Randomize colors").clicked() {
            particles.randomize_colors(rules.colors.len());
        }

        if ui.button("Reset attractions").clicked() {
            rules.reset_attractions();
        }

        let mut method = rules.acceleration_method;
        let before = method;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut method, AccelerationMethod::R1, "R1");
            ui.selectable_value(&mut method, AccelerationMethod::R2, "R2");
            ui.selectable_value(&mut method, AccelerationMethod::R3, "R3");
            ui.selectable_value(&mut method, AccelerationMethod::Deg90, "Deg90");
            ui.selectable_value(&mut method, AccelerationMethod::Attr, "Attr");
            ui.selectable_value(&mut method, AccelerationMethod::Planets, "Planets");
        });
        if method != before {
            rules.set_acceleration_method(method);
        }

        ui.add_space(10.);
        ui.label("Visual Settings");

        ui.add(
            egui::Slider::new(&mut particles.particle_size, 1.0..=100.0)
                .text("particle_size")
                .clamp_to_range(false),
        );

        ui.horizontal(|ui| {
            ui.selectable_value(&mut particles.shape, Shape::Circle, "Circle");
            ui.selectable_value(&mut particles.shape, Shape::Square, "Square");
        });

        ui.add_space(50.);

        let mut camera_settings = camera_settings.single_mut();
        ui.label("Camera Settings");
        ui.horizontal(|ui| {
            ui.label("Pan Speed");
            ui.add(egui::DragValue::new(&mut camera_settings.pan_speed).speed(0.01));
            ui.label("Scroll Speed");
            ui.add(egui::DragValue::new(&mut camera_settings.scroll_speed).speed(0.01));
        });
    });
}
