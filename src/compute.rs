use std::borrow::Cow;

use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        graph::CameraDriverLabel,
        render_graph::{self, RenderGraph, RenderGraphApp, RenderLabel, ViewNodeRunner},
        render_resource::{
            binding_types::{storage_buffer, uniform_buffer},
            encase::impl_matrix,
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BufferUsages,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor, ComputePipeline,
            Pipeline, PipelineCache, ShaderStages, ShaderType, StorageBuffer, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ViewUniform, ViewUniforms},
        Extract, Render, RenderApp, RenderSet,
    },
};

use crate::{
    camera::ParticleCamera,
    data::{AttractionRules, Particle, Particles, SimulationSettings, COLORS},
    draw::{DrawParticleLabel, DrawParticleNode, DrawParticlePipeline},
};

const WORKGROUP_SIZE: u32 = 64;

pub struct ComputePlugin;

impl Plugin for ComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<ParticleCamera>::default())
            .add_systems(Startup, setup);

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

        // bevy_mod_debugdump::print_render_graph(app);

        // draw particles setup
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GpuBuffers>();
        render_app.init_resource::<ParticleBindGroupLayouts>();
        render_app.init_resource::<ParticlePipeline>();
        // draw particles setup
        render_app.init_resource::<DrawParticlePipeline>();
    }
}

#[derive(Resource)]
struct TestParticles(Particles);

fn setup(mut commands: Commands) {
    let mut particles = Particles::new();
    particles.change_particle_count(10_000, 100., 100., 3);
    commands.insert_resource(TestParticles(particles));
    let mut settings = SimulationSettings::default();
    settings.randomize_attractions();
    commands.insert_resource(settings);
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
    pub bounds_x: f32,
    pub bounds_y: f32,

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

#[derive(ShaderType, Default)]
pub struct GpuParticles {
    #[size(runtime)]
    pub particles: Vec<Particle>,
}

#[derive(Resource)]
pub struct ParticlesInfo {
    pub particle_count: usize,
}

#[derive(Resource, Default)]
pub struct GpuBuffers {
    pub particles: StorageBuffer<GpuParticles>,
    pub rules: UniformBuffer<GpuSettings>,
}

fn extract_particle_related_things(
    mut commands: Commands,
    particles: Extract<Res<TestParticles>>,
    settings: Extract<Res<SimulationSettings>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut buffers: ResMut<GpuBuffers>,
    time: Extract<Res<Time<Virtual>>>,
) {
    if particles.is_changed() || particles.is_added() {
        info!("Uploading particles");
        let mut buffer = StorageBuffer::from(GpuParticles {
            // this clone would suck but later this particle initialization should happen on the gpu anyway
            particles: particles.0.particles.clone(),
        });
        buffer.add_usages(BufferUsages::VERTEX);
        buffer.write_buffer(&device, &queue);
        buffers.particles = buffer;
        commands.insert_resource(ParticlesInfo {
            particle_count: particles.0.particles.len(),
        });
    }

    let colors = std::array::from_fn(|i| COLORS[i].to_f32_array().into());
    let matrix = ColorMatrix::from_array(settings.matrix);

    let settings = GpuSettings {
        delta_time: time.delta_seconds(),
        particle_count: particles.0.particles.len() as u32,
        min_distance: settings.min_distance,
        max_distance: settings.max_distance,
        max_velocity: settings.max_velocity,
        velocity_half_life: settings.velocity_half_life,
        force_factor: settings.force_factor,
        bounds_x: settings.min_max_x,
        bounds_y: settings.min_max_y,

        color_count: settings.color_count as u32,
        max_color_count: COLORS.len() as u32,
        colors,
        matrix,
    };
    commands.insert_resource(settings);

    let mut buffer = UniformBuffer::from(settings);
    buffer.add_usages(BufferUsages::VERTEX);
    buffer.write_buffer(&device, &queue);
    buffers.rules = buffer;
}

#[derive(Resource)]
pub struct ParticlePipeline {
    update_velocity: CachedComputePipelineId,
    update_position: CachedComputePipelineId,
}

impl FromWorld for ParticlePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layouts = world.resource::<ParticleBindGroupLayouts>();

        let shader = world.load_asset("particle.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();
        let update_velocity = pipeline_cache.queue_compute_pipeline(
            bevy::render::render_resource::ComputePipelineDescriptor {
                label: None,
                layout: layouts.to_vec(),
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "update_velocity".into(),
            },
        );

        let update_position = pipeline_cache.queue_compute_pipeline(
            bevy::render::render_resource::ComputePipelineDescriptor {
                label: None,
                layout: layouts.to_vec(),
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "update_position".into(),
            },
        );

        ParticlePipeline {
            update_velocity,
            update_position,
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
                ShaderStages::COMPUTE | ShaderStages::VERTEX,
                (
                    storage_buffer::<GpuParticles>(false),
                    uniform_buffer::<GpuSettings>(false),
                    // WARNING: this is only ok with one camera. multiple cameras not supported
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
            buffers.rules.binding().unwrap(),
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
        let pipeline = world.resource::<ParticlePipeline>();
        let gpu_settings = world.resource::<GpuSettings>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        let Some(update_velocity) = pipeline_cache.get_compute_pipeline(pipeline.update_velocity)
        else {
            return Ok(());
        };

        let Some(update_position) = pipeline_cache.get_compute_pipeline(pipeline.update_position)
        else {
            return Ok(());
        };

        let workgroup_count =
            (gpu_settings.particle_count as f32 / WORKGROUP_SIZE as f32).ceil() as u32;

        pass.set_bind_group(0, &bind_groups[0], &[0]);

        pass.set_pipeline(update_velocity);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        pass.set_pipeline(update_position);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        Ok(())
    }
}
