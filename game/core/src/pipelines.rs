use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
    },
};

/// Counts the GPU pipelines of the render world for the main world, so the
/// loading screen can wait until the shaders are compiled.
pub struct PipelinesPlugin;

impl Plugin for PipelinesPlugin {
    fn build(&self, app: &mut App) {
        let pipelines = Pipelines::default();
        app.insert_resource(pipelines.clone());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(pipelines)
                .add_systems(Render, Pipelines::count.in_set(RenderSystems::Cleanup));
        }
    }
}

/// How many GPU pipelines exist and how many are still being compiled.
#[derive(Resource, Clone, Debug, Default)]
pub struct Pipelines(Arc<Counts>);

#[derive(Debug, Default)]
struct Counts {
    total: AtomicUsize,
    pending: AtomicUsize,
}

impl Pipelines {
    /// Pipelines that were counted as `total` with `pending` of them still
    /// compiling.
    #[cfg(any(test, feature = "test-support"))]
    pub fn counted(total: usize, pending: usize) -> Self {
        let pipelines = Self::default();
        pipelines.0.total.store(total, Ordering::Relaxed);
        pipelines.0.pending.store(pending, Ordering::Relaxed);
        pipelines
    }

    pub fn total(&self) -> usize {
        self.0.total.load(Ordering::Relaxed)
    }

    /// Pipelines that are queued or being compiled. Failed ones do not count.
    pub fn pending(&self) -> usize {
        self.0.pending.load(Ordering::Relaxed)
    }

    fn count(pipelines: Res<Pipelines>, cache: Res<PipelineCache>) {
        let (mut total, mut pending) = (0, 0);
        for pipeline in cache.pipelines() {
            total += 1;
            if matches!(
                pipeline.state,
                CachedPipelineState::Queued | CachedPipelineState::Creating(_)
            ) {
                pending += 1;
            }
        }
        pipelines.0.total.store(total, Ordering::Relaxed);
        pipelines.0.pending.store(pending, Ordering::Relaxed);
    }
}
