use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{NodeRunError, RenderGraphContext, ViewNode, ViewNodeRunner, RenderLabel, RenderGraphApp},
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingResource, BindingType, BufferBindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, Operations, PipelineCache,
            PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
            ShaderStages, ShaderType, TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
};
use bevy::ecs::query::QueryItem;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct SimpleOutlineLabel;

pub struct SimpleOutlinePlugin;

impl Plugin for SimpleOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<SimpleOutlineSettings>::default(),
            UniformComponentPlugin::<SimpleOutlineSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<SimpleOutlineNode>>(Core3d, SimpleOutlineLabel)
            .add_render_graph_edges(
                Core3d,
                (Node3d::Tonemapping, SimpleOutlineLabel, Node3d::EndMainPassPostProcessing),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<SimpleOutlinePipeline>();
    }
}

#[derive(Default)]
struct SimpleOutlineNode;

impl ViewNode for SimpleOutlineNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static SimpleOutlineSettings,
        &'static bevy::render::extract_component::DynamicUniformIndex<SimpleOutlineSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _outline_settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        println!("SimpleOutlineNode running with threshold: {}", _outline_settings.edge_threshold);
        let pipeline = world.resource::<SimpleOutlinePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<bevy::render::extract_component::ComponentUniforms<SimpleOutlineSettings>>();
        let Some(uniform_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "simple_outline_bind_group",
            &pipeline.layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(post_process.source),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniform_binding.clone(),
                },
            ],
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("simple_outline_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct SimpleOutlinePipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for SimpleOutlinePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let layout = render_device.create_bind_group_layout(
            "simple_outline_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let shader = asset_server.load("shaders/simple_outline.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("simple_outline_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
pub struct SimpleOutlineSettings {
    pub outline_thickness: f32,
    pub outline_color: Vec4,
    pub edge_threshold: f32,
    pub texture_size: Vec2,
    pub intensity: f32,
    pub _pad: Vec3,
}

impl Default for SimpleOutlineSettings {
    fn default() -> Self {
        Self {
            outline_thickness: 2.0,
            outline_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            edge_threshold: 0.1,
            texture_size: Vec2::new(1920.0, 1080.0),
            intensity: 1.0,
            _pad: Vec3::ZERO,
        }
    }
}

impl SimpleOutlineSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.outline_thickness = thickness;
        self
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.outline_color = color;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.edge_threshold = threshold;
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_texture_size(mut self, width: f32, height: f32) -> Self {
        self.texture_size = Vec2::new(width, height);
        self
    }
}