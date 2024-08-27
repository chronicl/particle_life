use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use bevy::{
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

    pub particle_size: f32,

    pub new_particles: u32,
    pub initialized_particles: u32,

    pub color_count: u32,
    pub max_color_count: u32,
    pub colors: [Vec4; COLORS.len()],
    // This is actually a COLORS.len()xCOLORS.len() matrix of f32
    pub matrix: ColorMatrix,
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

#[derive(ShaderType, Default, Clone)]
pub struct GpuParticles {
    #[size(runtime)]
    pub particles: Vec<Particle>,
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
        }
    }
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

fn extract_particle_related_things(
    mut commands: Commands,
    settings: Extract<Res<SimulationSettings>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut buffers: ResMut<GpuBuffers>,
    time: Extract<Res<Time<Virtual>>>,
    mut events_reader: Extract<EventReader<ParticleEvent>>,
    mut todo: ResMut<Todo>,
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
        bounds: settings.bounds,

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
}

#[derive(Resource)]
pub struct ParticlePipelines {
    initialize_particles: CachedComputePipelineId,
    update_velocity: CachedComputePipelineId,
    update_position: CachedComputePipelineId,
    randomize_positions: CachedComputePipelineId,
    randomize_colors: CachedComputePipelineId,
}

impl FromWorld for ParticlePipelines {
    fn from_world(world: &mut World) -> Self {
        let layouts = world.resource::<ParticleBindGroupLayouts>();

        let shader = world.load_asset("particle.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();

        let new_compute_pipeline = |label: &'static str| {
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

        let initialize_particles = new_compute_pipeline("initialize_particles");
        let update_velocity = new_compute_pipeline("update_velocity");
        let update_position = new_compute_pipeline("update_position");
        let randomize_positions = new_compute_pipeline("randomize_positions");
        let randomize_colors = new_compute_pipeline("randomize_colors");

        ParticlePipelines {
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

        pass.set_pipeline(update_velocity);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        pass.set_pipeline(update_position);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        Ok(())
    }
}
