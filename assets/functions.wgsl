#define_import_path functions

#import types::settings;

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

fn cell_count() -> u32 {
    return settings.cell_count.x * settings.cell_count.y;
}

fn cell_index(position: vec2<f32>) -> u32 {
    let cell_2d = cell_index_2d(position);
    let cell_index = cell_2d.x + cell_2d.y * settings.cell_count.x;
    return cell_index;
}

fn cell_index_2d(position: vec2<f32>) -> vec2<u32> {
    // moving the position from [-bounds, bounds] to [0, 2 * bounds];
    let p = settings.bounds + position;
    return vec2<u32>(floor(p / settings.max_distance));
}

fn surrounding_cells(position: vec2<f32>) -> array<u32, 9> {
   let cell = cell_index_2d(position);
   let cells = settings.cell_count;
   let minus_x = rem_euclid(i32(cell.x) - 1, cells.x);
   let minus_y = rem_euclid(i32(cell.y) - 1, cells.y) * cells.x;
   let plus_x = (cell.x + 1) % cells.x;
   let plus_y = ((cell.y + 1) % cells.y) * cells.x;
   let middle_x = cell.x;
   let middle_y = cell.y * cells.x;

   return array(
        minus_x + minus_y,
        middle_x + minus_y,
        plus_x + minus_y,
        minus_x + middle_y,
        middle_x + middle_y,
        plus_x + middle_y,
        minus_x + plus_y,
        middle_x + plus_y,
        plus_x + plus_y,
   );
}

// This only works for [-1, modulo - 1].
// Everywhere else isn't implemented because' wgsl % has some really weird
// behaviour. -1 % 10 == -1, but if you instead use a runtime variable x that is equal to -1
// then x % 10 == 5, with all types being i32.
fn rem_euclid(n: i32, modulo: u32) -> u32 {
    if (n == -1) {
        return modulo - 1;
    } else {
        return u32(n);
    }
}
