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
    tailwind::RED_600,
    tailwind::ROSE_600,
    tailwind::SKY_600,
    tailwind::SLATE_600,
    tailwind::VIOLET_600,
    tailwind::TEAL_600,
    tailwind::YELLOW_600,
];

#[derive(Resource, Default, Debug, Clone)]
pub struct Particles {
    pub particles: Vec<Particle>,
    pub particle_size: f32,
    pub shape: Shape,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Shape {
    #[default]
    Circle,
    Square,
}

impl std::ops::Deref for Particles {
    type Target = Vec<Particle>;

    fn deref(&self) -> &Self::Target {
        &self.particles
    }
}

impl std::ops::DerefMut for Particles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.particles
    }
}

impl Particles {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            particle_size: 4.,
            shape: Shape::Circle,
        }
    }

    pub fn randomize_positions(&mut self, min_max_x: f32, min_max_y: f32) {
        let mut rng = rand::thread_rng();
        for particle in self.particles.iter_mut() {
            particle.position = Vec2::new(
                rng.gen_range(-min_max_x..min_max_x),
                rng.gen_range(-min_max_y..min_max_y),
            );
        }
    }

    pub fn randomize_colors(&mut self, colors: usize) {
        let mut rng = rand::thread_rng();
        for particle in self.particles.iter_mut() {
            particle.color = ColorId::new(rng.gen_range(0..colors) as u32);
        }
    }

    pub fn change_particle_count(
        &mut self,
        count: usize,
        min_max_x: f32,
        min_max_y: f32,
        color_count: usize,
    ) {
        let mut rng = rand::thread_rng();
        self.particles.resize_with(count, || {
            Particle::new_random(&mut rng, min_max_x, min_max_y, color_count)
        });
    }
}

#[derive(Component, ShaderType, Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: ColorId,
}

impl Particle {
    pub fn new_random(
        rng: &mut impl Rng,
        min_max_x: f32,
        min_max_y: f32,
        color_count: usize,
    ) -> Self {
        Self {
            position: Vec2::new(
                rng.gen_range(-min_max_x..min_max_x),
                rng.gen_range(-min_max_y..min_max_y),
            ),
            velocity: Vec2::ZERO,
            color: ColorId::new(rng.gen_range(0..color_count) as u32),
        }
    }
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct ColorId {
    pub id: u32,
}

impl ColorId {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct AttractionRules {
    pub colors: Vec<Color>,
    pub matrix: Vec<Vec<f32>>,
    pub max_distance: f32,
    pub min_distance: f32,
    pub max_velocity: f32,
    pub velocity_half_life: f32,
    // parameters are relative to max_distance, where max_distance is 1.0
    pub acceleration_fn: fn(relative_min_distance: f32, dpos: Vec2, attraction: f32) -> Vec2,
    pub acceleration_method: AccelerationMethod,
    pub force_factor: f32,
    pub min_max_x: f32,
    pub min_max_y: f32,
}

impl Default for AttractionRules {
    fn default() -> Self {
        Self {
            colors: vec![
                LinearRgba::RED.into(),
                LinearRgba::GREEN.into(),
                LinearRgba::BLUE.into(),
            ],
            matrix: vec![vec![0.0; 3]; 3],
            max_distance: 250.0,
            min_distance: 30.0,
            max_velocity: 1000.0,
            velocity_half_life: 0.043,
            acceleration_fn: acceleration,
            acceleration_method: AccelerationMethod::R1,
            force_factor: 1.,
            min_max_x: 1200.,
            min_max_y: 800.,
        }
    }
}

impl AttractionRules {
    pub fn new(colors: Vec<Color>) -> Self {
        let color_count = colors.len();
        Self {
            colors,
            matrix: vec![vec![0.0; color_count]; color_count],
            ..default()
        }
    }

    pub fn change_colors(&mut self, colors: Vec<Color>) {
        self.colors = colors;
        self.randomize_attractions();
    }

    /// The color's index is their ColorId
    pub fn new_random(colors: Vec<Color>) -> Self {
        let mut this = Self::new(colors);
        this.randomize_attractions();
        this
    }

    pub fn randomize_attractions(&mut self) {
        self.matrix = vec![vec![0.0; self.colors.len()]; self.colors.len()];
        let mut rng = rand::thread_rng();
        for color_a in 0..self.colors.len() {
            for color_b in 0..self.colors.len() {
                let attraction = rng.gen_range(-1.0..1.0);
                self.matrix[color_a][color_b] = attraction;
            }
        }
    }

    pub fn reset_attractions(&mut self) {
        self.matrix = vec![vec![0.0; self.colors.len()]; self.colors.len()];
    }

    pub fn get_attraction(&self, color_a: ColorId, color_b: ColorId) -> f32 {
        self.matrix[color_a.id as usize][color_b.id as usize]
    }

    pub fn get_color(&self, color_id: ColorId) -> Color {
        self.colors[color_id.id as usize]
    }

    pub fn set_acceleration_method(&mut self, method: AccelerationMethod) {
        self.acceleration_method = method;
        match method {
            AccelerationMethod::R1 => self.acceleration_fn = acceleration,
            AccelerationMethod::R2 => self.acceleration_fn = acceleration2,
            AccelerationMethod::R3 => self.acceleration_fn = acceleration3,
            AccelerationMethod::Deg90 => self.acceleration_fn = acceleration90_,
            AccelerationMethod::Attr => self.acceleration_fn = acceleration_attr,
            AccelerationMethod::Planets => self.acceleration_fn = planets,
        }
    }

    pub fn closest_wrapped_other_position(&self, pos: Vec2, other_pos: Vec2) -> Vec2 {
        let x_bound = self.min_max_x;
        let y_bound = self.min_max_y;
        // Figuring out if by wrapping the position of the other particle, it is closer to our particle.
        let wrapped_x = if other_pos.x > 0. {
            other_pos.x - 2. * x_bound
        } else {
            other_pos.x + 2. * x_bound
        };
        let wrapped_y = if other_pos.y > 0. {
            other_pos.y - 2. * y_bound
        } else {
            other_pos.y + 2. * y_bound
        };

        let other_x = if (pos.x - wrapped_x).abs() < (pos.x - other_pos.x).abs() {
            wrapped_x
        } else {
            other_pos.x
        };
        let other_y = if (pos.y - wrapped_y).abs() < (pos.y - other_pos.y).abs() {
            wrapped_y
        } else {
            other_pos.y
        };

        Vec2::new(other_x, other_y)
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

pub fn random_colors(count: usize) -> Vec<Color> {
    COLORS
        .choose_multiple(&mut rand::thread_rng(), count)
        .map(|c| (*c).into())
        .collect()
}
