use bevy::color::{palettes::tailwind, Srgba};
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const COLORS: &[Srgba] = &[
    tailwind::EMERALD_600,
    tailwind::BLUE_600,
    tailwind::PURPLE_600,
    tailwind::PINK_600,
    tailwind::AMBER_600,
    tailwind::TEAL_600,
    tailwind::CYAN_600,
    tailwind::FUCHSIA_600,
    tailwind::GREEN_600,
    tailwind::INDIGO_600,
    tailwind::LIME_600,
    tailwind::ROSE_600,
    tailwind::SKY_600,
    tailwind::ORANGE_600,
    tailwind::VIOLET_600,
    tailwind::YELLOW_600,
];

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Shape {
    #[default]
    Circle = 0,
    Square = 1,
}

#[derive(Component, ShaderType, Default, Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: ColorId,
    pub padding: u32,
}

#[derive(ShaderType, Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorId {
    pub id: u32,
}

impl ColorId {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct SimulationSettings {
    pub particle_count: usize,
    // x and y of bounds must be multiples of max_distance for our
    // grid spatial partitioning to work. Our grid spatial partitioning
    // is based on the assumption that only the cells surroundingthe particles
    // cell can affect the particle, which is the case when each cell has max_distance
    // width and height.
    bounds: UVec2,
    max_distance: u32,
    pub min_distance: u32,
    pub max_velocity: f32,
    pub velocity_half_life: f32,
    pub force_factor: f32,
    pub max_attractions: u32,
    pub acceleration_method: AccelerationMethod,

    pub color_count: usize,
    pub color_order: Vec<ColorId>,
    pub matrix: Vec<Vec<f32>>,

    // visual settings
    pub particle_size: f32,
    pub shape: Shape,
    pub circle_corners: u32,
    pub rgb: bool,
    pub rgb_speed: f32,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            particle_count: 9000,
            bounds: UVec2::new(3600, 2100),
            max_distance: 250,
            min_distance: 50,
            max_velocity: 1000.0,
            velocity_half_life: 0.043,
            force_factor: 1.,
            max_attractions: 10_000,
            acceleration_method: AccelerationMethod::R1,

            color_count: 4,
            color_order: (0..COLORS.len()).map(|i| ColorId::new(i as u32)).collect(),
            matrix: (0..COLORS.len())
                .map(|_| (0..COLORS.len()).map(|_| 0.).collect())
                .collect(),

            particle_size: 4.,
            shape: Shape::Circle,
            circle_corners: 16,
            rgb: false,
            rgb_speed: 1.
        }
    }
}

impl SimulationSettings {
    pub fn randomize_colors(&mut self) {
        self.color_order.shuffle(&mut rand::thread_rng());
    }

    pub fn randomize_attractions(&mut self) {
        self.matrix = (0..COLORS.len())
            .map(|_| {
                (0..COLORS.len())
                    .map(|_| rand::thread_rng().gen_range(-1.0..1.0))
                    .collect()
            })
            .collect();
    }

    pub fn reset_attractions(&mut self) {
        self.matrix = Default::default();
    }

    pub fn max_distance(&self) -> u32 {
        self.max_distance
    }

    pub fn update_max_distance(&mut self, max_distance: u32) {
        self.max_distance = max_distance;
        self.update_bounds(self.bounds);
    }

    pub fn bounds(&self) -> UVec2 {
        self.bounds
    }

    pub fn update_bounds(&mut self, bounds: UVec2) {
        // round to closest multiple of max_distance
        self.bounds = ((bounds.as_vec2() / self.max_distance as f32)
            .round()
            .as_uvec2())
            * self.max_distance;
    }

    pub fn cell_count(&self) -> UVec2 {
        // bounds are [-bounds, bounds]
        2 * self.bounds / self.max_distance
    }

    pub fn serialize(&self) -> String {
        let mut settings = self.clone();
        let color_count = settings.color_count;

        // removing unused rows and columns from matrix
        settings.matrix.truncate(color_count);
        for row in settings.matrix.iter_mut() {
            row.truncate(color_count);
        }

        // removing unused colors in color_order
        settings.color_order.truncate(color_count);

        // let pretty = PrettyConfig::new()
        //     .depth_limit(2)
        //     .separate_tuple_members(true)
        //     .enumerate_arrays(true);
        // to_string_pretty(&settings, pretty).expect("Serialization failed")
        serde_json::to_string(&settings).expect("Serialization failed")
    }

    pub fn deserialize(s: &str) -> Option<Self> {
        let mut settings: Self = serde_json::from_str(s).ok()?;

        // adding missing rows and columns to matrix
        for _ in settings.color_count..COLORS.len() {
            settings.matrix.push(vec![0.; COLORS.len()]);
        }
        for row in settings.matrix.iter_mut() {
            row.resize_with(COLORS.len(), Default::default);
        }

        // adding missing colors to color_order
        let colors = settings.color_order.clone();
        for i in 0..COLORS.len() {
            if !colors.contains(&ColorId::new(i as u32)) {
                settings.color_order.push(ColorId::new(i as u32));
            }
        }

        Some(settings)
    }

    pub fn get_color(&self, color_id: ColorId) -> Srgba {
        COLORS[color_id.id as usize]
    }
}

// TODO: implement on gpu side
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
