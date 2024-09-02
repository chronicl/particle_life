#define_import_path types

#import bevy_render::view::View

@group(0) @binding(0) var<storage, read_write> particles: Particles;
@group(0) @binding(1) var<uniform> settings: Settings;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<storage, read_write> sorted_indices: array<u32>;
// used for cell offsets, which are calculated by prefix sum
@group(0) @binding(4) var<storage, read_write> counter: array<atomic<u32>>;
@group(0) @binding(5)
var<storage, read_write> prefix_sum_reduction: array<atomic<u32>>;
@group(0) @binding(6)
var<storage, read_write> prefix_sum_index: array<atomic<u32>>;

struct Particles {
    particles: array<Particle>,
}

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    color: u32,
    padding: u32,
}

struct Settings {
    time: f32,
    delta_time: f32,
    particle_count: u32,
    min_distance: f32,
    max_distance: f32,
    max_velocity: f32,
    velocity_half_life: f32,
    force_factor: f32,
    bounds: vec2<f32>,
    max_attractions: u32,
    acceleration_method: u32,

    new_particles: u32,
    initialized_particles: u32,

    shape: u32,
    circle_corners: u32,
    particle_size: f32,
    rgb: u32,
    rgb_speed: f32,

    cell_count: vec2<u32>,
    seed: u32,

    color_count: u32,
    max_color_count: u32,
    colors: array<vec4<f32>, 16>,
    matrix: array<vec4<f32>, 5>,
}
