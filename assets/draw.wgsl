#import types::{settings, particles, view, counter};

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
    return color;
}
