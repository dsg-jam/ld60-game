use bevy::{
    camera::RenderTarget, image::ImageSampler, prelude::*, render::render_resource::TextureFormat,
};

/// An offscreen UI that renders into an image.
pub(crate) struct UiCanvas {
    pub(crate) image: Handle<Image>,
    pub(crate) camera: Entity,
}
impl UiCanvas {
    pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>, size: UVec2) -> Self {
        let mut image =
            Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
        image.sampler = ImageSampler::nearest();
        let image = images.add(image);
        let camera = commands
            .spawn((
                Camera2d,
                Camera {
                    order: -1,
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                },
                RenderTarget::Image(image.clone().into()),
            ))
            .id();
        Self { image, camera }
    }
}
