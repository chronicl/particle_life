#import types::{settings, particles, view, counter};

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) instance: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

const PI: f32 = 3.14159;

fn polar_to_cartesian(r: f32, theta: f32) -> vec2<f32> {
    return vec2<f32>(r * cos(theta), r * sin(theta));
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out : VertexOutput;

    var local_position: vec2<f32>;
    // Square
    if (settings.shape == 1) {
        let index = square_indices[input.index];
        local_position = settings.particle_size * square_vertices[index];
    } else if (settings.shape == 0) {
        if (input.index % 3 == 2) {
            local_position = vec2<f32>(0.);
        } else {
            let i = input.index / 3;
            let offset = input.index % 3;
            let angle = 2. * PI * f32(i + offset
            ) / f32(settings.circle_corners);
            local_position = polar_to_cartesian(settings.particle_size, angle);
        }

    }

    let particle = particles.particles[input.instance];
    let center = particle.position;

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

var<private> square_vertices: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1, -1),
    vec2<f32>(1, -1),
    vec2<f32>(-1, 1),
    vec2<f32>(1, 1),
);

var<private> square_indices: array<u32, 6> = array<u32, 6>(
    0, 1, 2,
    1, 3, 2
);
