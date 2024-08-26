use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::storage_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, CachedComputePipelineId, CachedPipelineState,
            ComputePassDescriptor, ComputePipeline, Pipeline, PipelineCache, ShaderStages,
            ShaderType, StorageBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        Extract, Render, RenderApp, RenderSet,
    },
};

use crate::data::{Particle, Particles};

pub struct ComputePlugin;

impl Plugin for ComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(ExtractSchedule, extract_particles_buffer)
            .add_systems(
                Render,
                prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ParticleLabel, ParticleNode::default());
        render_graph.add_node_edge(ParticleLabel, CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ParticlePipeline>();
    }
}

#[derive(Resource)]
struct TestParticles(Particles);

fn setup(mut commands: Commands) {
    let mut particles = Particles::new();
    particles.change_particle_count(10, 100., 100., 3);
    commands.insert_resource(TestParticles(particles));
}

#[derive(ShaderType)]
pub struct GpuParticles {
    #[size(runtime)]
    pub particles: Vec<Particle>,
}

#[derive(Resource)]
pub struct GpuParticleBuffers {
    pub particles: StorageBuffer<GpuParticles>,
}

fn extract_particles_buffer(
    mut commands: Commands,
    particles: Extract<Res<TestParticles>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    if particles.is_changed() || particles.is_added() {
        let mut buffer = StorageBuffer::from(GpuParticles {
            // this clone would suck but later this particle initialization should happen on the gpu anyway
            particles: particles.0.particles.clone(),
        });
        buffer.write_buffer(&device, &queue);
        commands.insert_resource(GpuParticleBuffers { particles: buffer });
    }
}

#[derive(Resource)]
pub struct ParticlePipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl FromWorld for ParticlePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "ParticlesData",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (storage_buffer::<GpuParticles>(false),),
            ),
        );

        let shader = world.load_asset("particle.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(
            bevy::render::render_resource::ComputePipelineDescriptor {
                label: None,
                layout: vec![bind_group_layout.clone()],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "update".into(),
            },
        );

        ParticlePipeline {
            bind_group_layout,
            pipeline,
        }
    }
}

#[derive(Resource)]
struct ParticleBindGroups([BindGroup; 1]);

fn prepare_bind_groups(
    mut commands: Commands,
    pipeline: Res<ParticlePipeline>,
    buffers: Res<GpuParticleBuffers>,
    render_device: Res<RenderDevice>,
) {
    let particles = buffers.particles.binding().unwrap();
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.bind_group_layout,
        &BindGroupEntries::sequential((particles,)),
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

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        if let CachedPipelineState::Ok(pipeline) =
            pipeline_cache.get_compute_pipeline_state(pipeline.pipeline)
        {
            let Pipeline::ComputePipeline(pipeline) = pipeline else {
                unreachable!()
            };

            pass.set_bind_group(0, &bind_groups.0[0], &[]);
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(1, 1, 1);
            println!("dispatching");
        }

        Ok(())
    }
}
