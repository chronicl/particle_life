#import bevy_render::view::View;
#import bevy_pbr::utils::{rand_vec2f, rand_range_u};

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    color: u32,
    padding: u32,
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
    bounds: vec2<f32>,

    particle_size: f32,

    new_particles: u32,
    initialized_particles: u32,

    color_count: u32,
    max_color_count: u32,
    colors: array<vec4<f32>, 18>,
    matrix: array<vec4<f32>, 5>,


}

@group(0) @binding(1) var<uniform> settings: Settings;
@group(0) @binding(2) var<uniform> view: View;

@compute @workgroup_size(64)
fn randomize_positions(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    var i = global_id.x;
    let p = &particles.particles[global_id.x];
    (*p).position = (2. * rand_vec2f(&i) - 1.) * settings.bounds;
}

@compute @workgroup_size(64)
fn randomize_colors(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    var i = global_id.x;
    let p = &particles.particles[global_id.x];
    (*p).color = rand_range_u(settings.color_count, &i);
}


@compute @workgroup_size(64)
fn initialize_particles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.new_particles) {
        return;
    }

    var index = settings.initialized_particles + global_id.x;

    let p = &particles.particles[index];
    (*p).velocity = vec2<f32>(0.);
    (*p).position = (2. * rand_vec2f(&index) - 1.) * settings.bounds;
    (*p).color = rand_range_u(settings.color_count, &index);
}

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
        let other_position = closest_wrapped_other_position(particle.position, other.position, settings.bounds);

        let relative_position = other_position - particle.position;
        let distance_squared = dot(relative_position, relative_position);

        if distance_squared == 0. || distance_squared > settings.max_distance * settings.max_distance {
            continue;
        }

        let attraction = get_matrix_value(particle.color, other.color);

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
        force = a * (1. - abs(1. + rmin - 2. * dist) / (1. - rmin));
    }
    return pos * force / dist;
}

fn closest_wrapped_other_position(pos: vec2<f32>, other_pos: vec2<f32>, bounds: vec2<f32>) -> vec2<f32> {
    var other = other_pos;

    var wrapped: vec2<f32>;
    if (other_pos.x > 0.) {
        wrapped.x = other.x - 2. * bounds.x;
    } else {
        wrapped.x = other.x + 2. * bounds.x;
    }
    if (other_pos.y > 0.) {
       wrapped.y = other.y - 2. * bounds.y;
    } else {
       wrapped.y = other.y + 2. * bounds.y;
    }

    if abs(pos.x - wrapped.x) < abs(pos.x - other.x) {
        other.x = wrapped.x;
    }
    if abs(pos.y - wrapped.y) < abs(pos.y - other.y) {
        other.y = wrapped.y;
    }

    return other;
}

fn get_matrix_value(x: u32, y: u32) -> f32 {
    // var s = settings;
    let flat_index = x + y * settings.max_color_count;
    let index = flat_index / 4;
    let offset = flat_index % 4;
    return settings.matrix[index][offset];
}

@compute @workgroup_size(64)
fn update_position(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    let particle = &particles.particles[global_id.x];
    (*particle).position += (*particle).velocity * settings.delta_time;

    let p = particles.particles[global_id.x];
    if p.position.x > settings.bounds.x {
        (*particle).position.x -= 2. * settings.bounds.x;
        } else if p.position.x < -settings.bounds.x {
        (*particle).position.x += 2. * settings.bounds.x;
    }
    if p.position.y > settings.bounds.y {
        (*particle).position.y -= 2. * settings.bounds.y;
        } else if p.position.y < -settings.bounds.y {
        (*particle).position.y += 2. * settings.bounds.y;
    }
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

    var s: f32 = settings.particle_size;
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
    var i = input.instance;
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
    // return vec4<f32>(settings.velocity_half_life, 0., 0., 0.);
    return color;
}
