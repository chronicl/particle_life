use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::WindowMode,
};
use bevy_egui::{
    egui::{self, color_picker::show_color, Layout},
    EguiClipboard, EguiContexts,
};

use crate::{
    camera::CameraSettings,
    data::{AccelerationMethod, Shape, SimulationSettings, COLORS},
    events::ParticleEvent,
};

pub fn ui(
    mut contexts: EguiContexts,
    mut settings: ResMut<SimulationSettings>,
    mut camera_settings: Query<&mut CameraSettings>,
    diagnostic: Res<DiagnosticsStore>,
    mut time: ResMut<Time<Virtual>>,
    mut event_writer: EventWriter<ParticleEvent>,
    mut clipboard: ResMut<EguiClipboard>,
    mut window: Query<&mut Window>,
) {
    let mut window = window.single_mut();

    egui::Window::new("Settings")
        .scroll([false, true])
        .default_height(650.)
        .default_width(260.)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
                    if let Some(value) = fps.smoothed() {
                        ui.label(format!("FPS: {value:.2}"));
                    }
                }

                ui.with_layout(
                    Layout::right_to_left(bevy_egui::egui::Align::Center),
                    |ui| {
                        if ui.button("Toggle Fullscreen").clicked() {
                            window.mode = match window.mode {
                                WindowMode::Windowed => WindowMode::BorderlessFullscreen,
                                WindowMode::BorderlessFullscreen => WindowMode::Windowed,
                                _ => WindowMode::Windowed,
                            }
                        }
                    },
                );
            });

            let mut relative_speed = time.relative_speed();
            let before = relative_speed;
            ui.add(
                egui::Slider::new(&mut relative_speed, 0.0..=10.)
                    .text("simulation speed")
                    .clamp_to_range(false),
            );
            if relative_speed != before {
                time.set_relative_speed(relative_speed);
            }

            ui.add(
                egui::Slider::new(&mut settings.particle_count, 0..=300_000)
                    .text("particle count")
                    .clamp_to_range(false),
            );

            ui.add(
                egui::Slider::new(&mut settings.min_distance, 0..=200)
                    .text("min distance")
                    .clamp_to_range(false),
            );
            let mut max_distance = settings.max_distance();
            ui.add(
                egui::Slider::new(&mut max_distance, 100..=1000)
                    .text("max distance")
                    .clamp_to_range(false),
            );
            settings.update_max_distance(max_distance);
            ui.add(
                egui::Slider::new(&mut settings.max_velocity, 1.0..=1000.0)
                    .text("max velocity")
                    .clamp_to_range(false),
            );
            ui.add(
                egui::Slider::new(&mut settings.velocity_half_life, 0.001..=2.0)
                    .text("velocity half life"),
            );
            ui.add(
                egui::Slider::new(&mut settings.force_factor, 0.0..=30.0)
                    .text("force scale")
                    .clamp_to_range(false),
            );
            let mut bounds = settings.bounds();
            ui.add(
                egui::Slider::new(&mut bounds.x, 100..=30_000)
                    .text("bounds x")
                    .clamp_to_range(false),
            );
            ui.add(
                egui::Slider::new(&mut bounds.y, 100..=30_000)
                    .text("bounds y")
                    .clamp_to_range(false),
            );
            settings.update_bounds(bounds);

            ui.add(
                egui::Slider::new(&mut settings.max_attractions, 1..=30_000)
                    .text("max attractions")
                    .clamp_to_range(false),
            );

            ui.add(
                egui::Slider::new(&mut settings.color_count, 1..=COLORS.len())
                    .text("color count")
                    .clamp_to_range(false),
            );

            if settings.color_count < 11 {
                ui.add_space(10.);

                let color_ui = |ui: &mut egui::Ui, color: Srgba| {
                    let color: Color = color.into();
                    let color = color.to_srgba().to_u8_array();
                    show_color(
                        ui,
                        egui::Rgba::from_srgba_unmultiplied(color[0], color[1], color[2], color[3]),
                        egui::Vec2::new(20., 20.),
                    );
                };

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        color_ui(ui, Srgba::NONE);
                        for color in 0..settings.color_count {
                            color_ui(ui, COLORS[settings.color_order[color].id as usize]);
                        }
                    });

                    for i in 0..settings.color_count {
                        ui.vertical(|ui| {
                            color_ui(ui, COLORS[settings.color_order[i].id as usize]);

                            for j in 0..settings.color_count {
                                ui.add(
                                    egui::DragValue::new(&mut settings.matrix[i][j])
                                        .speed(0.01)
                                        .custom_formatter(|v, _| format!("{:.2}", v)),
                                );
                            }
                        });
                    }
                });
            }

            if ui.button("Randomize attractions").clicked() {
                settings.randomize_attractions();
            }

            if ui.button("Randomize positions").clicked() {
                event_writer.send(ParticleEvent::RandomizePositions);
            }

            if ui.button("Randomize colors").clicked() {
                event_writer.send(ParticleEvent::RandomizeColors);
            }

            if ui.button("Reset attractions").clicked() {
                settings.reset_attractions();
            }

            let mut method = settings.acceleration_method;
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
                settings.acceleration_method = method;
            }

            ui.add_space(10.);
            ui.label("Visual Settings");

            if ui.button("Randomize color palette").clicked() {
                settings.randomize_colors();
            }

            ui.add(
                egui::Slider::new(&mut settings.particle_size, 1.0..=100.0)
                    .text("particle_size")
                    .clamp_to_range(false),
            );

            ui.add(
                egui::Slider::new(&mut settings.circle_corners, 8..=128)
                    .text("circle_corners")
                    .clamp_to_range(false),
            );

            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.shape, Shape::Circle, "Circle");
                ui.selectable_value(&mut settings.shape, Shape::Square, "Square");
            });

            // settings.rgb
            ui.checkbox(&mut settings.rgb, "RGB");
            ui.add(egui::Slider::new(&mut settings.rgb_speed, 0.1..=10.).text("RGB speed"));

            if ui.button("Copy settings to clipboard").clicked() {
                clipboard.set_contents(&settings.serialize());
            };

            if ui.button("Paste settings from clipboard").clicked() {
                if let Some(s) = clipboard.get_contents() {
                    if let Some(s) = SimulationSettings::deserialize(&s) {
                        *settings = s;
                        event_writer.send(ParticleEvent::RandomizeColors);
                        event_writer.send(ParticleEvent::RandomizePositions);
                    }
                }
            }

            ui.add_space(10.);

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
