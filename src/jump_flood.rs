use bevy::ecs::query::QueryItem;
use bevy::render::render_resource::{BufferInitDescriptor, BufferUsages, Extent3d};
use bevy::render::view::{ViewDepthTexture, ViewTarget};
use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingResource, BindingType, BufferBindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, Operations, PipelineCache,
            PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
            TextureUsages, TextureView, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        RenderApp,
    },
};
use std::num::NonZeroU64;
fn create_readable_scene_texture(
    device: &bevy::render::renderer::RenderDevice,
    source: &bevy::render::view::ViewTarget,
) -> bevy::render::render_resource::Texture {
    let scene_texture = source.main_texture();
    let size = scene_texture.size();

    device.create_texture(&TextureDescriptor {
        label: Some("scene_readable"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: scene_texture.format(),
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct JumpFloodOutlineLabel;

pub struct JumpFloodOutlinePlugin;

impl Plugin for JumpFloodOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<OutlineSettings>::default(),
            UniformComponentPlugin::<OutlineSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<JumpFloodOutlineNode>>(
                Core3d,
                JumpFloodOutlineLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPassPostProcessing,
                    JumpFloodOutlineLabel,
                    Node3d::Upscaling,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<JumpFloodPipeline>();
    }
}

#[derive(Default)]
struct JumpFloodOutlineNode;

impl ViewNode for JumpFloodOutlineNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static OutlineSettings,
        &'static bevy::render::extract_component::DynamicUniformIndex<OutlineSettings>,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, outline_settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<JumpFloodPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let initial_scene_texture = view_target.main_texture();
        let initial_scene_view = view_target.main_texture_view();

        let view_size = initial_scene_texture.size();
        let max_dimension = view_size.width.max(view_size.height);
        let steps = (max_dimension as f32).log2().ceil() as u32;

        let temp_textures = self.create_temp_textures(render_context.render_device(), view_size);
        let view_entity = graph.view_entity();

        let view_target = view_target.main_texture().size();

        self.edge_detection_pass(
            render_context,
            &pipeline,
            pipeline_cache,
            initial_scene_view,          // Input Scena (&TextureView)
            &temp_textures.ping_texture, // Scrive il seed su PING
            outline_settings,
            settings_index.index(),
            world,
            view_entity,
        )?;

        // Fase 2: Jump Flood (PING/PONG)
        let result_is_in_ping = self.jump_flood_passes(
            render_context,
            &pipeline,
            pipeline_cache,
            &temp_textures,
            steps,
            settings_index.index(),
            world,
        )?;

        let final_jfa_map = if result_is_in_ping {
            &temp_textures.ping_texture
        } else {
            &temp_textures.pong_texture
        };

        // Fase Finale: Composizione (Scena + JFA -> Destinazione finale del ViewTarget)
        let post_process_write = view_target.post_process_write();

        self.final_outline_pass(
            render_context,
            &pipeline,
            pipeline_cache,
            initial_scene_view,             // Input Scena (&TextureView)
            final_jfa_map,                  // Mappa distanza finale
            post_process_write.destination, // Output Finale (&TextureView)
            settings_index.index(),
            world,
        )?;

        Ok(())
    }
}
// ---

impl JumpFloodOutlineNode {
    fn create_temp_textures(&self, device: &RenderDevice, size: Extent3d) -> TempTextures {
        let texture_descriptor = TextureDescriptor {
            label: Some("jump_flood_temp"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg32Float, // RG per archiviare coordinate 2D
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING, 
            view_formats: &[],
        };

        let ping_texture = device.create_texture(&texture_descriptor);
        let pong_texture = device.create_texture(&texture_descriptor);

        TempTextures {
            ping_texture: ping_texture.create_view(&Default::default()),
            pong_texture: pong_texture.create_view(&Default::default()),
        }
    }

    fn edge_detection_pass(
        &self,
        render_context: &mut RenderContext,
        pipeline: &JumpFloodPipeline,
        pipeline_cache: &PipelineCache,
        source: &TextureView,
        output: &TextureView,
        _settings: &OutlineSettings,
        settings_offset: u32,
        world: &World,
        view_entity: Entity,
    ) -> Result<(), NodeRunError> {
        let Some(edge_pipeline) = pipeline_cache.get_render_pipeline(pipeline.edge_pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms =
            world.resource::<bevy::render::extract_component::ComponentUniforms<OutlineSettings>>();
        let Some(uniform_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // Ottieni ViewTarget e depth view
        let view_target = world
            .get::<ViewTarget>(view_entity)
            .expect("ViewTarget non trovato");
let depth_texture = world.get::<ViewDepthTexture>(view_entity)
    .expect("ViewDepthTexture non trovata");

let depth_view = depth_texture.view();

        let bind_group = render_context.render_device().create_bind_group(
            "edge_detection_bind_group",
            &pipeline.basic_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(source),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(depth_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: uniform_binding.clone(),
                },
            ],
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("edge_detection_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(edge_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_offset]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }

    fn jump_flood_passes(
        &self,
        render_context: &mut RenderContext,
        pipeline: &JumpFloodPipeline,
        pipeline_cache: &PipelineCache,
        temp_textures: &TempTextures,
        steps: u32,
        settings_offset: u32,
        world: &World,
    ) -> Result<bool, NodeRunError> {
        let Some(jfa_pipeline) = pipeline_cache.get_render_pipeline(pipeline.jfa_pipeline_id)
        else {
            return Ok(false);
        };

        let settings_uniforms =
            world.resource::<bevy::render::extract_component::ComponentUniforms<OutlineSettings>>();
        let Some(uniform_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(false);
        };

        // Inizializzazione per il double buffering:
        // L'Edge Detection ha scritto il seed in PING (input per la prima iterazione)
        let mut current_input = &temp_textures.ping_texture;
        let mut current_output = &temp_textures.pong_texture;

        // `true` se il risultato è in PING, `false` se è in PONG
        let mut result_is_in_ping = true;

        // Iterazione JFA (dal passo più grande al più piccolo)
        for i in (0..steps).rev() {
            let step_size = 1 << i;
            let step_f32 = step_size as f32;

            // Inizializza il buffer dello step
            let step_data: [f32; 8] = [step_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
            let step_buffer =
                render_context
                    .render_device()
                    .create_buffer_with_data(&BufferInitDescriptor {
                        label: Some(&format!("jfa_step_{}", step_size)),
                        contents: bytemuck::cast_slice(&step_data),
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });

            // Crea Bind Group con l'input corrente
            let bind_group = render_context.render_device().create_bind_group(
                "jfa_bind_group_{}",
                &pipeline.jfa_layout,
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(current_input), // Lettura
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&pipeline.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: uniform_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: step_buffer.as_entire_binding(),
                    },
                ],
            );

            {
                // **CORREZIONE**: Assicurati che il RenderPass traccia solo l'output
                let mut render_pass =
                    render_context.begin_tracked_render_pass(RenderPassDescriptor {
                        label: Some(&format!("jfa_pass_{}", step_size)),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: current_output, // Scrittura
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                render_pass.set_render_pipeline(jfa_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[settings_offset]);
                render_pass.draw(0..3, 0..1);
            } // Il RenderPass termina qui, rilasciando l'uso esclusivo di current_output

            // **SWAP** per la prossima iterazione
            std::mem::swap(&mut current_input, &mut current_output);
            result_is_in_ping = !result_is_in_ping;
        }

        // Restituisce dove si trova il risultato finale
        Ok(result_is_in_ping)
    }

    fn final_outline_pass(
        &self,
        render_context: &mut RenderContext,
        pipeline: &JumpFloodPipeline,
        pipeline_cache: &PipelineCache,
        source: &TextureView,       // Input originale (view_target.source)
        distance_map: &TextureView, // Risultato JFA (ping o pong)
        output: &TextureView,       // Output finale (view_target.destination)
        settings_offset: u32,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(final_pipeline) = pipeline_cache.get_render_pipeline(pipeline.final_pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms =
            world.resource::<bevy::render::extract_component::ComponentUniforms<OutlineSettings>>();
        let Some(uniform_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            "final_outline_bind_group",
            &pipeline.final_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(source), // Input 1: Scena
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniform_binding.clone(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(distance_map), // Input 2: Mappa Distanza
                },
            ],
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("final_outline_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: output, // Output: Scrive sul destination
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(final_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_offset]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
struct TempTextures {
    // Ho rimosso 'seed_texture' e 'distance_texture' qui perché PING e PONG le assorbono.
    ping_texture: TextureView,
    pong_texture: TextureView,
}

#[derive(Resource)]
struct JumpFloodPipeline {
    basic_layout: BindGroupLayout,
    jfa_layout: BindGroupLayout,
    final_layout: BindGroupLayout,
    sampler: Sampler,
    edge_pipeline_id: CachedRenderPipelineId,
    jfa_pipeline_id: CachedRenderPipelineId,
    final_pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for JumpFloodPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let basic_layout = render_device.create_bind_group_layout(
            "edge_detection_layout",
            &[
                // Binding 0: screen texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Binding 1: sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Binding 2: depth texture (ERA UNIFORM, ORA È TEXTURE!)
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                // Binding 3: uniform settings (SPOSTATO DA 2 A 3dd
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true, // Se usi dynamic offset
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        // Layout per JFA (4 bindings: Texture, Sampler, Settings Uniform, Step Uniform)
        let jfa_layout = render_device.create_bind_group_layout(
            "jump_flood_jfa_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0, // Input Texture (Ping/Pong)
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1, // Sampler
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2, // Settings Uniform
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3, // Step Uniform
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false, // Non è necessario l'offset dinamico
                        min_binding_size: Some(NonZeroU64::new(32).unwrap()), // float/vec4 sono 16/32 byte
                    },
                    count: None,
                },
            ],
        );

        // Layout per final pass (4 bindings: Scena, Sampler, Settings, Mappa Distanza)
        let final_layout = render_device.create_bind_group_layout(
            "jump_flood_final_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0, // Scena Input
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1, // Sampler
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2, // Settings Uniform
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3, // Mappa Distanza Input
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let edge_shader = asset_server.load("shaders/jump_flood_edge.wgsl");
        let jfa_shader = asset_server.load("shaders/jump_flood_algorithm.wgsl");
        let final_shader = asset_server.load("shaders/jump_flood_final.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();

        // Edge Pipeline (usa basic_layout)
        let edge_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("edge_detection_pipeline".into()),
            layout: vec![basic_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: edge_shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rg32Float,
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

        // JFA Pipeline (usa jfa_layout)
        let jfa_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("jfa_pipeline".into()),
            layout: vec![jfa_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: jfa_shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rg32Float,
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

        // Final Pipeline (usa final_layout)
        let final_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("final_outline_pipeline".into()),
            layout: vec![final_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: final_shader,
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
            basic_layout,
            final_layout,
            jfa_layout,
            sampler,
            edge_pipeline_id,
            jfa_pipeline_id,
            final_pipeline_id,
        }
    }
}

// ... (OutlineSettings è OK)
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
pub struct OutlineSettings {
    pub outline_thickness: f32,
    pub outline_color: Vec4,
    pub edge_threshold: f32,
    pub depth_threshold: f32,
    pub texture_size: Vec2,
    pub _pad: Vec2,
}

impl Default for OutlineSettings {
    fn default() -> Self {
        Self {
            outline_thickness: 2.0,
            outline_color: Vec4::new(0.0, 0.0, 0.0, 1.0), // Nero
            edge_threshold: 0.1,
            depth_threshold: 0.1,
            texture_size: Vec2::new(1920.0, 1080.0),
            _pad: Vec2::ZERO,
        }
    }
}

impl OutlineSettings {
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
}
