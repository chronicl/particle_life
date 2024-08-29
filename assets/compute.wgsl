#import bevy_pbr::utils::{rand_vec2f, rand_range_u};
#import types::{settings, particles, counter, sorted_indices};
#import functions::{closest_wrapped_other_position, get_matrix_value, acceleration, cell_index, cell_count, surrounding_cells};

var<workgroup> item_sum: atomic<i32>;

// counter[ci] will countain the amount of particles in cell index ci
@compute @workgroup_size(64)
fn count_particles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    let index = global_id.x;
    let ci = cell_index(particles.particles[index].position);
    atomicAdd(&counter[ci], 1u);
}

// counter[ci] will be the count of all particles
// in the cells with cell index smaller than or equal to ci
@compute @workgroup_size(1)
fn cell_offsets() {
    let count = cell_count();
    for (var i = 1u; i < count; i++) {
        counter[i] = counter[i] + counter[i - 1];
    }
    let total_cells = cell_count();
    counter[total_cells] = settings.particle_count;
}

// counter[ci] will be the starting index of the particles of cell index
// ci in the array sorted_indices.
// Which also means that (counter[ci + 1]).max(total_particle_count) is the end.
@compute @workgroup_size(64)
fn sort_particles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    let index = global_id.x;
    let ci = cell_index(particles.particles[index].position);
    let sorted_index = atomicSub(&counter[ci], 1u) - 1;
    sorted_indices[sorted_index] = index;
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
fn update_velocity(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    let index = global_id.x;

    let particle = particles.particles[index];
    let particle_ref = &particles.particles[index];
   (*particle_ref).velocity *= pow(0.5, settings.delta_time / settings.velocity_half_life);

    var surrounding = surrounding_cells(particle.position);

    for (var j = 0u; j < 9; j++) {
        let ci = surrounding[j];
        let start = counter[ci];
        let end = counter[ci + 1];

        for (var i = start; i < end; i++) {
            let pi = sorted_indices[i];
            let other = particles.particles[pi];
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
}

@compute @workgroup_size(64)
fn update_velocitya(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= settings.particle_count) {
        return;
    }

    let index = global_id.x;

    let particle = particles.particles[index];
    let particle_ref = &particles.particles[index];
   (*particle_ref).velocity *= pow(0.5, settings.delta_time / settings.velocity_half_life);

    for (var i = 0u; i < settings.particle_count; i++) {
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
