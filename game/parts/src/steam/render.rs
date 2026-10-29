use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::ExtractComponentPlugin,
        render_asset::RenderAssets,
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BufferInitDescriptor, BufferUsages, CachedComputePipelineId, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, ShaderStages, StorageTextureAccess,
            TextureFormat,
            binding_types::{texture_storage_3d, uniform_buffer},
            encase::UniformBuffer,
        },
        renderer::{RenderContext, RenderGraph, RenderGraphSystems},
        texture::GpuImage,
    },
};

use super::{GRID, Plume, PlumeParams};

/// Threads of a workgroup along each side, matching `plume.wesl`.
const WORKGROUP: u32 = 4;

pub(super) struct PlumeRenderPlugin;

impl Plugin for PlumeRenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "plume.wesl");
        app.add_plugins(ExtractComponentPlugin::<Plume>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, PlumePipeline::init)
            .add_systems(RenderGraph, Plume::draw.in_set(RenderGraphSystems::Begin));
    }
}

/// The compute pipeline that draws [`Plume`] densities.
#[derive(Resource)]
struct PlumePipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

impl PlumePipeline {
    fn init(
        mut commands: Commands,
        pipeline_cache: Res<PipelineCache>,
        asset_server: Res<AssetServer>,
    ) {
        let layout = BindGroupLayoutDescriptor::new(
            "plume_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<PlumeParams>(false),
                    texture_storage_3d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("plume_pipeline".into()),
            layout: vec![layout.clone()],
            shader: load_embedded_asset!(asset_server.as_ref(), "plume.wesl"),
            ..default()
        });
        commands.insert_resource(Self { layout, pipeline });
    }
}

impl Plume {
    /// Draws the density of every plume a camera sees.
    fn draw(
        plumes: Query<&Plume>,
        pipeline: Res<PlumePipeline>,
        pipeline_cache: Res<PipelineCache>,
        images: Res<RenderAssets<GpuImage>>,
        mut ctx: RenderContext,
    ) {
        let Some(compute) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
            return;
        };
        for plume in plumes.iter().filter(|plume| plume.seen) {
            let Some(image) = images.get(&plume.density) else {
                continue;
            };
            let mut params = UniformBuffer::new(Vec::new());
            if params.write(&plume.params).is_err() {
                continue;
            }
            let buffer = ctx
                .render_device()
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("plume_params"),
                    contents: params.as_ref(),
                    usage: BufferUsages::UNIFORM,
                });
            let bind_group = ctx.render_device().create_bind_group(
                "plume_bind_group",
                &pipeline_cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((buffer.as_entire_binding(), &image.texture_view)),
            );
            let mut pass = ctx
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("plume_pass"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(compute);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                GRID.x.div_ceil(WORKGROUP),
                GRID.y.div_ceil(WORKGROUP),
                GRID.z.div_ceil(WORKGROUP),
            );
        }
    }
}
