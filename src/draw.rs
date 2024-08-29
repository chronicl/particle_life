use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{self, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNodeRunner},
        render_resource::*,
        renderer::RenderContext,
        texture::BevyDefault,
        view::{ViewTarget, ViewUniformOffset},
        RenderApp,
    },
};

use crate::{
    camera::ParticleCamera,
    compute::{ParticleBindGroupLayouts, ParticleBindGroups, SHADER_DRAW},
    data::{Shape, SimulationSettings},
};

pub struct DrawPlugin;

impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        // We are borrowing the bind groups from the compute plugin.
        // So very little setup is needed here.
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_graph_node::<ViewNodeRunner<DrawParticleNode>>(Core2d, DrawParticleLabel)
            .add_render_graph_edge(Core2d, Node2d::Tonemapping, DrawParticleLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<DrawParticlePipeline>();
    }
}

#[derive(Resource)]
pub struct DrawParticlePipeline {
    pipeline: CachedRenderPipelineId,
}

impl FromWorld for DrawParticlePipeline {
    fn from_world(world: &mut World) -> Self {
        let layouts = world.resource::<ParticleBindGroupLayouts>();

        // Only loading it here instead of using the internal asset
        // because the internal asset doesn't hot reload (bug)
        let shader = world.load_asset("draw.wgsl");
        // let shader = SHADER_DRAW;

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_render_pipeline(
            bevy::render::render_resource::RenderPipelineDescriptor {
                label: None,
                layout: layouts.to_vec(),
                push_constant_ranges: Vec::new(),
                vertex: VertexState {
                    shader: shader.clone(),
                    entry_point: "vertex".into(),
                    shader_defs: vec![],
                    buffers: vec![],
                },
                fragment: Some(FragmentState {
                    shader: shader.clone(),
                    shader_defs: vec![],
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState { ..default() },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
            },
        );

        DrawParticlePipeline { pipeline }
    }
}

#[derive(RenderLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawParticleLabel;

#[derive(Default)]
pub struct DrawParticleNode;

impl render_graph::ViewNode for DrawParticleNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ParticleCamera,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _, uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = world.resource::<ParticleBindGroups>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<DrawParticlePipeline>();
        let settings = world.resource::<SimulationSettings>();

        let color_attachment = view_target.get_color_attachment();

        let mut pass = render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        if let CachedPipelineState::Ok(pipeline) =
            pipeline_cache.get_render_pipeline_state(pipeline.pipeline)
        {
            let Pipeline::RenderPipeline(pipeline) = pipeline else {
                unreachable!()
            };

            pass.set_bind_group(0, &bind_groups[0], &[uniform_offset.offset]);
            pass.set_pipeline(pipeline);
            match settings.shape {
                Shape::Circle => {
                    pass.draw(
                        0..settings.circle_corners * 3,
                        0..settings.particle_count as u32,
                    );
                }
                Shape::Square => {
                    pass.draw(0..6, 0..settings.particle_count as u32);
                }
            }
        }

        Ok(())
    }
}
