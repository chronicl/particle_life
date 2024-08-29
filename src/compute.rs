use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        graph::CameraDriverLabel,
        render_graph::{self, RenderGraph, RenderGraphApp, RenderLabel, ViewNodeRunner},
        render_resource::{
            binding_types::{storage_buffer, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BufferUsages,
            BufferVec, CachedComputePipelineId, CachedPipelineState, CommandEncoderDescriptor,
            ComputePassDescriptor, ComputePipeline, Maintain, Pipeline, PipelineCache,
            ShaderStages, ShaderType, StorageBuffer, UniformBuffer, UninitBufferVec,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ViewUniform, ViewUniforms},
        Extract, Render, RenderApp, RenderSet,
    },
};

pub const SHADER_FUNCTIONS: Handle<Shader> = Handle::weak_from_u128(4912569123382610166);
pub const SHADER_TYPES: Handle<Shader> = Handle::weak_from_u128(4923569123382510166);
pub const SHADER_COMPUTE: Handle<Shader> = Handle::weak_from_u128(4313569123342610166);
pub const SHADER_DRAW: Handle<Shader> = Handle::weak_from_u128(3913559123382610166);
pub const SHADER_PREFIX_SUM: Handle<Shader> = Handle::weak_from_u128(3913559123182610166);

fn load_shaders(app: &mut App) {
    load_internal_asset!(app, SHADER_TYPES, "../assets/types.wgsl", Shader::from_wgsl);
    load_internal_asset!(
        app,
        SHADER_FUNCTIONS,
        "../assets/functions.wgsl",
        Shader::from_wgsl
    );

    load_internal_asset!(app, SHADER_DRAW, "../assets/draw.wgsl", Shader::from_wgsl);
    load_internal_asset!(
        app,
        SHADER_COMPUTE,
        "../assets/compute.wgsl",
        Shader::from_wgsl
    );

    load_internal_asset!(
        app,
        SHADER_PREFIX_SUM,
        "../assets/prefix_sum.wgsl",
        Shader::from_wgsl
    );
}

use crate::{
    camera::ParticleCamera,
    data::{Particle, SimulationSettings, COLORS},
    draw::{DrawParticleLabel, DrawParticleNode, DrawParticlePipeline},
    events::ParticleEvent,
};

const WORKGROUP_SIZE: u32 = 64;

pub struct ComputePlugin;

impl Plugin for ComputePlugin {
    fn build(&self, app: &mut App) {
        load_shaders(app);

        app.add_plugins(ExtractComponentPlugin::<ParticleCamera>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(ExtractSchedule, extract_particle_related_things)
            .add_systems(
                Render,
                prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
            )
            .add_render_graph_node::<ViewNodeRunner<DrawParticleNode>>(Core2d, DrawParticleLabel)
            .add_render_graph_edge(Core2d, Node2d::Tonemapping, DrawParticleLabel);

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ParticleLabel, ParticleNode);
        render_graph.add_node_edge(ParticleLabel, CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(GpuBuffers::new());
        render_app.init_resource::<Todo>();
        render_app.init_resource::<ParticleBindGroupLayouts>();
        render_app.init_resource::<ParticlePipelines>();
        // draw particles setup
        render_app.init_resource::<DrawParticlePipeline>();
    }
}

#[derive(Resource, ShaderType, Default, Clone, Copy)]
pub struct GpuSettings {
    pub delta_time: f32,
    pub particle_count: u32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub max_velocity: f32,
    pub velocity_half_life: f32,
    pub force_factor: f32,
    pub bounds: Vec2,
    pub cell_count: UVec2,

    pub particle_size: f32,

    pub new_particles: u32,
    pub initialized_particles: u32,

    pub color_count: u32,
    pub max_color_count: u32,
    pub colors: [Vec4; COLORS.len()],
    // This is actually a COLORS.len()xCOLORS.len() matrix of f32
    pub matrix: ColorMatrix,
    // prefix sum info
}

#[derive(ShaderType, Clone, Copy)]
pub struct ColorMatrix {
    pub matrix: [Vec4; COLORS.len() * COLORS.len() / 4 + 1],
}

impl Default for ColorMatrix {
    fn default() -> Self {
        Self {
            matrix: [Vec4::ZERO; COLORS.len() * COLORS.len() / 4 + 1],
        }
    }
}

impl ColorMatrix {
    pub fn from_array(array: [[f32; COLORS.len()]; COLORS.len()]) -> Self {
        let mut this = Self::default();
        for (y, row) in array.iter().enumerate() {
            for (x, value) in row.iter().enumerate() {
                this.set(x, y, *value);
            }
        }
        this
    }

    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        let flat_index = x + y * COLORS.len();
        let index = flat_index / 4;
        let offset = flat_index % 4;
        self.matrix[index][offset] = value.into();
    }
}

#[test]
fn test_color_matrix() {
    let matrix = ColorMatrix::from_array(std::array::from_fn(|i| {
        std::array::from_fn(|j| (i * COLORS.len() + j) as f32)
    }));
    println!("{:?}", matrix.matrix);
}

#[derive(Resource, Default)]
struct Todo {
    randomize_positions: AtomicBool,
    randomize_colors: AtomicBool,
}

impl Todo {
    fn set_randomize_positions(&self, value: bool) {
        self.randomize_positions.store(value, Ordering::Relaxed);
    }

    fn randomize_positions(&self) -> bool {
        self.randomize_positions.load(Ordering::Relaxed)
    }

    fn set_randomize_colors(&self, value: bool) {
        self.randomize_colors.store(value, Ordering::Relaxed);
    }

    fn randomize_colors(&self) -> bool {
        self.randomize_colors.load(Ordering::Relaxed)
    }
}

#[derive(Resource)]
pub struct GpuBuffers {
    /// settings.particle_count >= allocated_particles is ensured in extract_particle_related_things
    allocated_particles: usize,
    /// settings.particle_count == initialized_particles is ensured in ParticleNode
    initialized_particles: AtomicU32,
    waited: u32,
    pub particles: UninitBufferVec<Particle>,
    pub settings: UniformBuffer<GpuSettings>,
    pub sorted_indices: StorageBuffer<Vec<u32>>,

    // prefix sum buffers. used for calculating the cell offsets
    pub thread_blocks: u32,
    pub counter: StorageBuffer<Vec<u32>>,
    pub prefix_sum_reduction: StorageBuffer<Vec<u32>>,
    pub prefix_sum_index: StorageBuffer<u32>,
}

impl GpuBuffers {
    pub fn new() -> Self {
        Self {
            allocated_particles: 0,
            initialized_particles: AtomicU32::new(0),
            waited: 0,
            particles: UninitBufferVec::new(
                BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            ),
            settings: UniformBuffer::default(),
            sorted_indices: StorageBuffer::default(),

            thread_blocks: 0,
            counter: StorageBuffer::default(),
            prefix_sum_reduction: StorageBuffer::default(),
            prefix_sum_index: StorageBuffer::default(),
        }
    }
}

fn extract_particle_related_things(
    mut commands: Commands,
    settings: Extract<Res<SimulationSettings>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut buffers: ResMut<GpuBuffers>,
    time: Extract<Res<Time<Virtual>>>,
    mut events_reader: Extract<EventReader<ParticleEvent>>,
    todo: Res<Todo>,
) {
    commands.insert_resource(settings.clone());

    for event in events_reader.read() {
        match event {
            ParticleEvent::RandomizePositions => {
                todo.set_randomize_positions(true);
            }
            ParticleEvent::RandomizeColors => {
                todo.set_randomize_colors(true);
            }
        }
    }

    buffers.waited += 1;
    if buffers.allocated_particles < settings.particle_count {
        info!("Replacing buffer");
        let new_particles_offset = buffers.particles.len();

        if new_particles_offset == 0 {
            buffers.particles.add();
            buffers.particles.add();
            buffers.particles.write_buffer(&device);
        }

        let mut new_buffer = UninitBufferVec::<Particle>::new(
            BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        );
        for _ in 0..settings.particle_count {
            new_buffer.add();
        }
        new_buffer.write_buffer(&device);
        let mut command_encoder =
            device.create_command_encoder(&CommandEncoderDescriptor { label: None });
        command_encoder.copy_buffer_to_buffer(
            buffers.particles.buffer().unwrap(),
            0,
            new_buffer.buffer().unwrap(),
            0,
            (std::mem::size_of::<Particle>() * new_particles_offset) as u64,
        );
        queue.submit([command_encoder.finish()]);
        device.poll(Maintain::Wait);

        buffers.particles = new_buffer;

        buffers.allocated_particles = settings.particle_count;
        buffers.waited = 0;
    }

    let colors = std::array::from_fn(|i| COLORS[i].to_f32_array().into());
    let matrix = ColorMatrix::from_array(settings.matrix);
    // println!("{:?}", matrix.matrix);

    let gpu_settings = GpuSettings {
        delta_time: time.delta_seconds(),
        particle_count: settings.particle_count as u32,
        min_distance: settings.min_distance,
        max_distance: settings.max_distance,
        max_velocity: settings.max_velocity,
        velocity_half_life: settings.velocity_half_life,
        force_factor: settings.force_factor,
        // Making the bounds a tiny bit smaller so that floating
        // point errors in the shader don't cause the grid cells to be one too many
        bounds: settings.bounds,
        cell_count: UVec2::new(
            (2. * settings.bounds.x / settings.max_distance) as u32,
            (2. * settings.bounds.y / settings.max_distance) as u32,
        ),

        particle_size: settings.particle_size,

        new_particles: (settings.particle_count as i32
            - buffers.initialized_particles.load(Ordering::Relaxed) as i32)
            .max(0) as u32,
        initialized_particles: buffers.initialized_particles.load(Ordering::Relaxed) as u32,

        color_count: settings.color_count as u32,
        max_color_count: COLORS.len() as u32,
        colors,
        matrix,
    };
    let mut buffer = UniformBuffer::from(gpu_settings);
    commands.insert_resource(gpu_settings);
    buffer.add_usages(BufferUsages::VERTEX);
    buffer.write_buffer(&device, &queue);
    buffers.settings = buffer;

    // TODO: these buffers should not be uploaded every frame.
    // They should be wiped gpu side and only uploaded once their size needs to change.
    let mut buffer = StorageBuffer::from(vec![0u32; settings.particle_count]);
    buffer.write_buffer(&device, &queue);
    buffers.sorted_indices = buffer;

    let size = gpu_settings.cell_count.x * gpu_settings.cell_count.y;
    let thread_blocks = (size as f32 / 256 as f32).ceil() as u32;
    buffers.thread_blocks = thread_blocks;
    let mut buffer = StorageBuffer::from(vec![0u32; size as usize]);
    buffer.write_buffer(&device, &queue);
    buffers.counter = buffer;
    let mut buffer = StorageBuffer::from(vec![0u32; thread_blocks as usize]);
    buffer.write_buffer(&device, &queue);
    buffers.prefix_sum_reduction = buffer;
    let mut buffer = StorageBuffer::from(0u32);
    buffer.write_buffer(&device, &queue);
    buffers.prefix_sum_index = buffer;
}

#[test]
fn test_cell_index() {
    let settings = SimulationSettings::default();
    let position = -settings.bounds + Vec2::new(1.5 * settings.max_distance, settings.max_distance);
    let cell_index = cell_index(&settings, position);
    println!("{}", cell_index);
}

fn cell_index(settings: &SimulationSettings, position: Vec2) -> u32 {
    let p = settings.bounds + position;
    println!("{:?}", p);
    let cells_x = (2. * settings.bounds.x / settings.max_distance).ceil() as u32;
    println!("{}", cells_x);
    let cell_index_2d = (p / settings.max_distance).floor();
    println!("{:?}", cell_index_2d);
    cell_index_2d.x as u32 + cell_index_2d.y as u32 * cells_x
}

#[test]
fn test_surrounding_cells() {
    let cells = UVec2::new(3, 3);
    let cell = UVec2::new(2, 2);
    let surrounding = surrounding_cells(cell, cells);
    println!("{:?}", surrounding);
    // assert_eq!(surrounding, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn test_manual_rem_euclid() {
    fn rem_euclid(a: i32, b: i32) -> i32 {
        (a % b + b) % b
    }
    assert_eq!(rem_euclid(0, 1), 0);
    assert_eq!(rem_euclid(1, 1), 0);
    assert_eq!(rem_euclid(-1, 1), 0);
    assert_eq!(rem_euclid(2, 1), 0);
    assert_eq!(rem_euclid(-2, 1), 0);

    assert_eq!(rem_euclid(0, 2), 0);
    assert_eq!(rem_euclid(1, 2), 1);
    assert_eq!(rem_euclid(-4, 3), 2);
    assert_eq!(rem_euclid(2, 2), 0);
    assert_eq!(rem_euclid(-2, 2), 0);
}

fn surrounding_cells(cell: UVec2, cells: UVec2) -> [u32; 9] {
    let minus_x = (cell.x as i32 - 1).rem_euclid(cells.x as i32) as u32;
    let minus_y = (cell.y as i32 - 1).rem_euclid(cells.y as i32) as u32;
    let plus_x = (cell.x as i32 + 1).rem_euclid(cells.x as i32) as u32;
    let plus_y = (cell.y as i32 + 1).rem_euclid(cells.y as i32) as u32;
    let middle_x = cell.x;
    let middle_y = cell.y;

    let below = minus_y * cells.x;
    let middle = middle_y * cells.x;
    let above = plus_y * cells.x;

    [
        minus_x + below,
        middle_x + below,
        plus_x + below,
        minus_x + middle,
        middle_x + middle,
        plus_x + middle,
        minus_x + above,
        middle_x + above,
        plus_x + above,
    ]
}

#[derive(Resource)]
pub struct ParticlePipelines {
    prefix_sum: CachedComputePipelineId,
    count_particles: CachedComputePipelineId,
    cell_offsets: CachedComputePipelineId,
    sort_particles: CachedComputePipelineId,
    initialize_particles: CachedComputePipelineId,
    update_velocity: CachedComputePipelineId,
    update_position: CachedComputePipelineId,
    randomize_positions: CachedComputePipelineId,
    randomize_colors: CachedComputePipelineId,
}

impl FromWorld for ParticlePipelines {
    fn from_world(world: &mut World) -> Self {
        let layouts = world.resource::<ParticleBindGroupLayouts>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let new_compute_pipeline = |label: &'static str, shader: &Handle<Shader>| {
            pipeline_cache.queue_compute_pipeline(
                bevy::render::render_resource::ComputePipelineDescriptor {
                    label: Some(label.into()),
                    layout: layouts.to_vec(),
                    push_constant_ranges: Vec::new(),
                    shader: shader.clone(),
                    shader_defs: vec![],
                    entry_point: label.into(),
                },
            )
        };

        // Only loading it here instead of using the internal asset
        // because the internal asset doesn't hot reload (bug)
        // let shader = world.load_asset("compute.wgsl");
        let shader = SHADER_COMPUTE;
        let prefix_sum_shader = SHADER_PREFIX_SUM;
        let prefix_sum = new_compute_pipeline("main", &prefix_sum_shader);
        let count_particles = new_compute_pipeline("count_particles", &shader);
        let cell_offsets = new_compute_pipeline("cell_offsets", &shader);
        let sort_particles = new_compute_pipeline("sort_particles", &shader);
        let initialize_particles = new_compute_pipeline("initialize_particles", &shader);
        let update_velocity = new_compute_pipeline("update_velocity", &shader);
        let update_position = new_compute_pipeline("update_position", &shader);
        let randomize_positions = new_compute_pipeline("randomize_positions", &shader);
        let randomize_colors = new_compute_pipeline("randomize_colors", &shader);

        ParticlePipelines {
            prefix_sum,
            count_particles,
            cell_offsets,
            sort_particles,
            initialize_particles,
            update_velocity,
            update_position,
            randomize_positions,
            randomize_colors,
        }
    }
}

#[derive(Resource, Deref)]
pub struct ParticleBindGroupLayouts(pub [BindGroupLayout; 1]);

impl FromWorld for ParticleBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "ParticlesLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE | ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                (
                    storage_buffer::<Vec<Particle>>(false),
                    uniform_buffer::<GpuSettings>(false),
                    uniform_buffer::<ViewUniform>(true),
                    storage_buffer::<Vec<u32>>(false),
                    storage_buffer::<Vec<u32>>(false),
                    storage_buffer::<Vec<u32>>(false),
                    storage_buffer::<u32>(false),
                ),
            ),
        );

        ParticleBindGroupLayouts([bind_group_layout])
    }
}

#[derive(Resource, Deref)]
pub struct ParticleBindGroups(pub [BindGroup; 1]);

fn prepare_bind_groups(
    mut commands: Commands,
    layouts: Res<ParticleBindGroupLayouts>,
    buffers: Res<GpuBuffers>,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
) {
    let bind_group = render_device.create_bind_group(
        None,
        &layouts[0],
        &BindGroupEntries::sequential((
            buffers.particles.binding().unwrap(),
            buffers.settings.binding().unwrap(),
            view_uniforms.uniforms.binding().unwrap(),
            buffers.sorted_indices.binding().unwrap(),
            buffers.counter.binding().unwrap(),
            buffers.prefix_sum_reduction.binding().unwrap(),
            buffers.prefix_sum_index.binding().unwrap(),
        )),
    );
    commands.insert_resource(ParticleBindGroups([bind_group]));
}

#[derive(RenderLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ParticleLabel;

#[derive(Default)]
struct ParticleNode;

impl render_graph::Node for ParticleNode {
    fn run<'w>(
        &self,
        graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = world.resource::<ParticleBindGroups>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ParticlePipelines>();
        let gpu_settings = world.resource::<GpuSettings>();
        let settings = world.resource::<SimulationSettings>();
        let buffers = world.resource::<GpuBuffers>();
        let todo = world.resource::<Todo>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        macro_rules! get_pipeline {
            ($name:ident) => {
                match pipeline_cache.get_compute_pipeline(pipeline.$name) {
                    Some(pipeline) => pipeline,
                    None => return Ok(()),
                }
            };
            () => {};
        }

        let prefix_sum = get_pipeline!(prefix_sum);
        let count_particles = get_pipeline!(count_particles);
        let cell_offsets = get_pipeline!(cell_offsets);
        let sort_particles = get_pipeline!(sort_particles);
        let initialize_particles = get_pipeline!(initialize_particles);
        let update_velocity = get_pipeline!(update_velocity);
        let update_position = get_pipeline!(update_position);
        let randomize_positions = get_pipeline!(randomize_positions);
        let randomize_colors = get_pipeline!(randomize_colors);

        let workgroup_count =
            (settings.particle_count as f32 / WORKGROUP_SIZE as f32).ceil() as u32;

        pass.set_bind_group(0, &bind_groups[0], &[0]);

        if todo.randomize_positions() {
            pass.set_pipeline(randomize_positions);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
            todo.set_randomize_positions(false);
        }

        if todo.randomize_colors() {
            pass.set_pipeline(randomize_colors);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
            todo.set_randomize_colors(false);
        }

        // Spawn new particles if neededd
        if gpu_settings.new_particles > 0 {
            let w = (gpu_settings.new_particles as f32 / WORKGROUP_SIZE as f32).ceil() as u32;
            pass.set_pipeline(initialize_particles);
            pass.dispatch_workgroups(w, 1, 1);
            buffers
                .initialized_particles
                .fetch_add(gpu_settings.new_particles, Ordering::Relaxed);
        }

        pass.set_pipeline(count_particles);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        // pass.set_pipeline(cell_offsets);
        // pass.dispatch_workgroups(1, 1, 1);

        pass.set_pipeline(prefix_sum);
        pass.dispatch_workgroups(buffers.thread_blocks, 1, 1);

        pass.set_pipeline(sort_particles);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        pass.set_pipeline(update_velocity);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        pass.set_pipeline(update_position);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        Ok(())
    }
}
