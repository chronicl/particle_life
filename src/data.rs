use std::array;
use std::sync::atomic::AtomicBool;

use bevy::color::{palettes::tailwind, Srgba};
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use rand::seq::SliceRandom;
use rand::Rng;

pub const COLORS: &[Srgba] = &[
    tailwind::AMBER_600,
    tailwind::BLUE_600,
    tailwind::CYAN_600,
    tailwind::EMERALD_600,
    tailwind::FUCHSIA_600,
    tailwind::GREEN_600,
    tailwind::INDIGO_600,
    tailwind::LIME_600,
    tailwind::ORANGE_600,
    tailwind::PINK_600,
    tailwind::PURPLE_600,
    tailwind::GRAY_600,
    tailwind::ROSE_600,
    tailwind::SKY_600,
    tailwind::SLATE_600,
    tailwind::VIOLET_600,
    tailwind::TEAL_600,
    tailwind::YELLOW_600,
];

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Shape {
    #[default]
    Circle,
    Square,
}

#[derive(Component, ShaderType, Default, Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: ColorId,
    pub padding: u32,
}

#[derive(ShaderType, Default, Debug, Clone, Copy)]
pub struct ColorId {
    pub id: u32,
}

impl ColorId {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccelerationMethod {
    R1,
    R2,
    R3,
    Deg90,
    Attr,
    Planets,
}

pub fn random_colors(count: usize) -> Vec<Color> {
    COLORS
        .choose_multiple(&mut rand::thread_rng(), count)
        .map(|c| (*c).into())
        .collect()
}

#[derive(Resource, Debug, Clone)]
pub struct SimulationSettings {
    pub particle_count: usize,
    pub max_distance: f32,
    pub min_distance: f32,
    pub max_velocity: f32,
    pub velocity_half_life: f32,
    pub force_factor: f32,
    pub bounds: Vec2,

    pub color_count: usize,
    pub color_order: [ColorId; COLORS.len()],
    pub matrix: [[f32; COLORS.len()]; COLORS.len()],

    // visual settings
    pub particle_size: f32,
    pub shape: Shape,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            particle_count: 1000,
            color_count: 3,
            color_order: array::from_fn(|i| ColorId::new(i as u32)),
            matrix: Default::default(),

            max_distance: 250.0,
            min_distance: 50.0,
            max_velocity: 1000.0,
            velocity_half_life: 0.043,
            force_factor: 1.,
            bounds: Vec2::new(1250., 750.),

            particle_size: 4.,
            shape: Shape::Square,
        }
    }
}

impl SimulationSettings {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn randomize_colors(&mut self) {
        self.color_order.shuffle(&mut rand::thread_rng());
    }

    pub fn randomize_attractions(&mut self) {
        self.matrix =
            array::from_fn(|_| array::from_fn(|_| rand::thread_rng().gen_range(-1.0..1.0)));
    }

    pub fn reset_attractions(&mut self) {
        self.matrix = Default::default();
    }

    pub fn get_color(&self, color_id: ColorId) -> Srgba {
        COLORS[color_id.id as usize]
    }
}

fn acceleration(rmin: f32, dpos: Vec2, a: f32) -> Vec2 {
    let dist = dpos.length();
    let force = if dist < rmin {
        dist / rmin - 1.
    } else {
        a * (1. - (1. + rmin - 2. * dist).abs() / (1. - rmin))
    };
    dpos * force / dist
}

fn acceleration2(rmin: f32, dpos: Vec2, a: f32) -> Vec2 {
    let dist = dpos.length();
    let force = if dist < rmin {
        dist / rmin - 1.
    } else {
        a * (1. - (1. + rmin - 2. * dist).abs() / (1. - rmin))
    };
    dpos * force / (dist * dist)
}

fn acceleration3(rmin: f32, dpos: Vec2, a: f32) -> Vec2 {
    let dist = dpos.length();
    let force = if dist < rmin {
        dist / rmin - 1.
    } else {
        a * (1. - (1. + rmin - 2. * dist).abs() / (1. - rmin))
    };
    dpos * force / (dist * dist * dist)
}

fn acceleration90_(rmin: f32, dpos: Vec2, a: f32) -> Vec2 {
    let dist = dpos.length();
    let force = a * (1. - dist);
    Vec2::new(-dpos.y, dpos.x) * force / dist
}

fn acceleration_attr(rmin: f32, dpos: Vec2, a: f32) -> Vec2 {
    let dist = dpos.length();
    let force = 1. - dist;
    let angle = -a * std::f32::consts::PI;
    Vec2::new(
        angle.cos() * dpos.x + angle.sin() * dpos.y,
        -angle.sin() * dpos.x + angle.cos() * dpos.y,
    ) * force
        / dist
}

fn planets(rmin: f32, dpos: Vec2, a: f32) -> Vec2 {
    let dist = dpos.length().max(0.01);
    dpos * 0.01 / (dist * dist * dist)
}
