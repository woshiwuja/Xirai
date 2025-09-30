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
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, Operations,
            PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
            ShaderStages, ShaderType, TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
};
use bevy::ecs::query::QueryItem;
use bevy_image::BevyDefault;
const SHADER_ASSET_PATH: &str = "shaders/pastel.wgsl";

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct PastelloLabel;
pub struct PastelloPostProcessPlugin;

impl Plugin for PastelloPostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<PastelloSettings>::default(),
            UniformComponentPlugin::<PastelloSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<PastelloNode>>(Core3d, PastelloLabel)
            .add_render_graph_edges(
                Core3d,
                (Node3d::Tonemapping, PastelloLabel, Node3d::EndMainPassPostProcessing),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<PastelloPipeline>();
    }
}


#[derive(Default)]
struct PastelloNode;

impl ViewNode for PastelloNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static PastelloSettings,
        &'static bevy::render::extract_component::DynamicUniformIndex<PastelloSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _pastello_settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pastello_pipeline = world.resource::<PastelloPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pastello_pipeline.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<bevy::render::extract_component::ComponentUniforms<PastelloSettings>>();
        let Some(uniform_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "pastello_bind_group",
            &pastello_pipeline.layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(post_process.source),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pastello_pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniform_binding.clone(),
                },
            ],
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("pastello_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct PastelloPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for PastelloPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let layout = render_device.create_bind_group_layout(
            "pastello_bind_group_layout",
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

        let shader = asset_server.load(SHADER_ASSET_PATH);

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("pastello_pipeline".into()),
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
pub struct PastelloSettings {
    pub intensity: f32,
    pub color_levels: f32,
    pub outline_strength: f32,
    pub outline_threshold: f32,

    pub saturation_boost: f32,
    pub brightness_boost: f32,
    pub contrast_reduction: f32,
    pub edge_softness: f32,

    pub texture_size: Vec2,
    pub _pad0: Vec2,

    pub use_custom_palette: u32,
    pub palette_size: u32,
    pub _pad1: Vec2,

    pub palette_colors: [Vec4; 8],
}

impl Default for PastelloSettings {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            color_levels: 8.0,
            outline_strength: 1.5,
            outline_threshold: 0.12,

            saturation_boost: 0.8,
            brightness_boost: 1.2,
            contrast_reduction: 0.8,
            edge_softness: 0.5,

            texture_size: Vec2::new(1920.0, 1080.0),
            _pad0: Vec2::ZERO,

            use_custom_palette: 0,
            palette_size: 0,
            _pad1: Vec2::ZERO,

            palette_colors: [Vec4::ZERO; 8],
        }
    }
}

impl PastelloSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pastello_palette(mut self) -> Self {
        self.use_custom_palette = 1;
        self.palette_size = 6;

        self.palette_colors[0] = Vec4::new(1.0, 0.8, 0.8, 1.0); // Rosa pastello
        self.palette_colors[1] = Vec4::new(0.8, 1.0, 0.8, 1.0); // Verde pastello
        self.palette_colors[2] = Vec4::new(0.8, 0.8, 1.0, 1.0); // Blu pastello
        self.palette_colors[3] = Vec4::new(1.0, 1.0, 0.8, 1.0); // Giallo pastello
        self.palette_colors[4] = Vec4::new(1.0, 0.8, 1.0, 1.0); // Magenta pastello
        self.palette_colors[5] = Vec4::new(0.8, 1.0, 1.0, 1.0); // Ciano pastello

        self
    }

    pub fn cartoon_style(mut self) -> Self {
        self.color_levels = 4.0;
        self.outline_strength = 2.5;
        self.outline_threshold = 0.08;
        self.contrast_reduction = 0.6;
        self
    }

    pub fn watercolor_style(mut self) -> Self {
        self.color_levels = 12.0;
        self.outline_strength = 0.8;
        self.outline_threshold = 0.2;
        self.saturation_boost = 0.6;
        self.edge_softness = 0.8;
        self
    }

    pub fn with_texture_size(mut self, width: f32, height: f32) -> Self {
        self.texture_size = Vec2::new(width, height);
        self
    }
}

// Sistema opzionale per aggiornare le dimensioni texture automaticamente
pub fn update_pastello_texture_size(
    mut settings: Query<&mut PastelloSettings>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if let Ok(window) = windows.single() {
        let size = Vec2::new(window.width(), window.height());
        for mut setting in settings.iter_mut() {
            if setting.texture_size != size {
                setting.texture_size = size;
            }
        }
    }
}