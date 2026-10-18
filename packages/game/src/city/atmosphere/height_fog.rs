use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{Core3dSystems, FullscreenShader, schedule::Core3d, tonemapping::tonemapping},
    post_process::bloom::bloom,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, Operations,
            PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType,
            binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{
            ExtractedView, ViewDepthStencilTexture, ViewTarget, ViewUniform, ViewUniformOffset,
            ViewUniforms,
        },
    },
};

pub(super) struct HeightFogPlugin;

impl Plugin for HeightFogPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "height_fog.wesl");
        app.add_plugins((
            ExtractComponentPlugin::<HeightFog>::default(),
            UniformComponentPlugin::<HeightFog>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<FogPipeline>>()
            .add_systems(RenderStartup, FogPipeline::init)
            .add_systems(Render, FogPipeline::prepare.in_set(RenderSystems::Prepare))
            .add_systems(
                Core3d,
                HeightFog::draw
                    .in_set(Core3dSystems::PostProcess)
                    .before(bloom)
                    .before(tonemapping),
            );
    }
}

/// Haze that is thickest at the ground and thins out with height, like the
/// air over a city.
///
/// Goes on a camera. It covers everything the camera sees, the sky
/// included, so far away buildings and the sky behind them fade alike.
#[derive(Component, ExtractComponent, ShaderType, Clone, Copy, Debug)]
#[extract_app(RenderApp)]
pub(super) struct HeightFog {
    /// Linear color the haze glows in.
    pub(super) color: Vec4,
    /// How much a meter of air at the ground hides.
    pub(super) density: f32,
    /// How quickly the haze thins out, per meter of height.
    pub(super) falloff: f32,
    /// Height of the ground the density is measured at.
    pub(super) ground: f32,
}

/// The render pipeline for [`HeightFog`].
#[derive(Resource)]
struct FogPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    shader: Handle<Shader>,
    fullscreen: FullscreenShader,
}

/// The pipeline of a camera's [`HeightFog`], for its target format.
#[derive(Component)]
struct FogPipelineId(CachedRenderPipelineId);

impl SpecializedRenderPipeline for FogPipeline {
    type Key = TextureFormat;

    fn specialize(&self, format: TextureFormat) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("height_fog_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

impl FogPipeline {
    fn init(
        mut commands: Commands,
        render_device: Res<RenderDevice>,
        asset_server: Res<AssetServer>,
        fullscreen: Res<FullscreenShader>,
    ) {
        let layout = BindGroupLayoutDescriptor::new(
            "height_fog_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<HeightFog>(true),
                ),
            ),
        );
        commands.insert_resource(Self {
            layout,
            sampler: render_device.create_sampler(&SamplerDescriptor::default()),
            shader: load_embedded_asset!(asset_server.as_ref(), "height_fog.wesl"),
            fullscreen: fullscreen.clone(),
        });
    }

    fn prepare(
        mut commands: Commands,
        pipeline_cache: Res<PipelineCache>,
        pipeline: Res<Self>,
        mut pipelines: ResMut<SpecializedRenderPipelines<Self>>,
        views: Query<(Entity, &ExtractedView), With<HeightFog>>,
    ) {
        for (entity, view) in &views {
            let id = pipelines.specialize(&pipeline_cache, &pipeline, view.target_format);
            commands.entity(entity).insert(FogPipelineId(id));
        }
    }
}

impl HeightFog {
    fn draw(
        view: ViewQuery<(
            &ViewUniformOffset,
            &ViewTarget,
            &ViewDepthStencilTexture,
            &DynamicUniformIndex<HeightFog>,
            &FogPipelineId,
        )>,
        pipeline: Res<FogPipeline>,
        pipeline_cache: Res<PipelineCache>,
        view_uniforms: Res<ViewUniforms>,
        fog_uniforms: Res<ComponentUniforms<HeightFog>>,
        mut ctx: RenderContext,
    ) {
        let (view_offset, view_target, depth, fog_index, pipeline_id) = view.into_inner();
        let (Some(render_pipeline), Some(view_binding), Some(fog_binding), Some(depth_view)) = (
            pipeline_cache.get_render_pipeline(pipeline_id.0),
            view_uniforms.uniforms.binding(),
            fog_uniforms.uniforms().binding(),
            depth.attachment.depth_stencil_views().depth_only_view(),
        ) else {
            return;
        };
        let post_process = view_target.post_process_write();
        let bind_group = ctx.render_device().create_bind_group(
            "height_fog_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                view_binding,
                depth_view,
                post_process.source,
                &pipeline.sampler,
                fog_binding,
            )),
        );
        let mut pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("height_fog_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: post_process.destination,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                ..default()
            });
        pass.set_pipeline(render_pipeline);
        pass.set_bind_group(0, &bind_group, &[view_offset.offset, fog_index.index()]);
        pass.draw(0..3, 0..1);
    }
}
