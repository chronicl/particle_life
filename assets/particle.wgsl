#import bevy_render::view::View;

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    color: u32
}

struct Particles {
    particles: array<Particle>
}

@group(0) @binding(0) var<storage, read_write> particles: Particles;

struct Settings {
    delta_time: f32,
    particle_count: u32,
    min_distance: f32,
    max_distance: f32,
    max_velocity: f32,
    velocity_half_life: f32,
    force_factor: f32,
    bounds_x: f32,
    bounds_y: f32,

    color_count: u32,
    max_color_count: u32,
    colors: array<vec4<f32>, 18>,
    matrix: array<vec4<f32>, 5>
}

fn get_matrix_value(settings: Settings, x: u32, y: u32) -> f32 {
    var s = settings;
    let flat_index = x + y * settings.color_count;
    let index = flat_index / 4;
    let offset = flat_index % 4;
    return s.matrix[index][offset];
}

@group(0) @binding(1) var<uniform> settings: Settings;
@group(0) @binding(2) var<uniform> view: View;

@compute @workgroup_size(64)
fn update_velocity(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }


    let particle = particles.particles[global_id.x];
    let particle_ref = &particles.particles[global_id.x];
   (*particle_ref).velocity *= pow(0.5, settings.delta_time / settings.velocity_half_life);


    for (var i = u32(0); i < settings.particle_count; i++) {
        let other = particles.particles[i];

        let relative_position = other.position - particle.position;
        let distance_squared = dot(relative_position, relative_position);

        if distance_squared == 0. || distance_squared > settings.max_distance * settings.max_distance {
            continue;
        }

        let attraction = get_matrix_value(settings, particle.color, other.color);

        let a = acceleration(settings.min_distance / settings.max_distance, relative_position / settings.max_distance, attraction);

        (*particle_ref).velocity += a * settings.max_distance * settings.force_factor * settings.delta_time;
    }
}

fn acceleration(rmin: f32, pos: vec2<f32>, a: f32) -> vec2<f32> {
    let dist = length(pos);
    var force: f32;
    if (dist < rmin) {
        force = dist / rmin - 1.;
    } else {
        force = a * (1. - abs(1. + rmin - 2. * dist) / (1. -rmin));
    }
    return pos * force / dist;
}

@compute @workgroup_size(64)
fn update_position(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    let particle = particles.particles[global_id.x];
    particles.particles[global_id.x].position += particle.velocity * settings.delta_time;
}

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) instance: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out : VertexOutput;

    var s: f32 =4.;
    var square_vertices= array<vec2<f32>, 4>(
        vec2<f32>(-s, -s),
        vec2<f32>(s, -s),
        vec2<f32>(-s, s),
        vec2<f32>(s, s),
    );

    var square_indices = array<u32, 6>(
        0, 1, 2,
        1, 3, 2
    );

    let particle = particles.particles[input.instance];
   let center = particle.position;

    let index = square_indices[input.index];
    let local_position = square_vertices[index];
    let view_position = vec4<f32>(local_position  + center, 0., 1.);
    let clip_position = view.clip_from_world * view_position;

    out.position = clip_position;
    out.color = settings.colors[particle.color];

    return out;
}

@fragment
fn fragment(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}
