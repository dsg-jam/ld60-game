mod crt;

use bevy::{
    asset::uuid::Uuid,
    camera::{ImageRenderTarget, NormalizedRenderTarget, RenderTarget, visibility::RenderLayers},
    image::ImageSampler,
    input::ButtonState,
    input_focus::AcquireFocus,
    picking::{
        PickingSystems,
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::{PrimaryWindow, WindowEvent, WindowRef},
};
use ld60_game_core::state::AppState;
use ld60_game_player::cursor::WantsArrow;

pub use self::crt::{CrtGlass, CrtLight, CrtMaterial};

/// Screens only show UI, which ignores render layers. Off the scene's layer,
/// their cameras skip culling every mesh in the city.
const CANVAS_LAYER: usize = 1;
/// Texels per UI pixel of a [`UiCanvas::spawn_print`] canvas. Read up close,
/// a sheet covers more screen pixels than its canvas has UI pixels.
const PRINT_SCALE: f32 = 2.0;

pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crt::CrtPlugin)
            .add_systems(
                First,
                ScreenSurface::drive_pointers.in_set(PickingSystems::Input),
            )
            .add_observer(ScreenSurface::add_pointer)
            .add_observer(ScreenSurface::keep_ui_focus);
    }
}

/// Ties a canvas camera or UI root to the entity that shows the canvas, so
/// they despawn with it.
#[derive(Component, Debug)]
#[relationship(relationship_target = Canvases)]
pub struct CanvasOf(Entity);

#[derive(Component, Debug, Default)]
#[relationship_target(relationship = CanvasOf, linked_spawn)]
pub struct Canvases(Vec<Entity>);

/// An offscreen UI that renders into an image, shown by `owner`.
pub struct UiCanvas {
    pub image: Handle<Image>,
    pub camera: Entity,
    pub size: UVec2,
    /// Texels per UI pixel.
    scale: f32,
    owner: Entity,
}

impl UiCanvas {
    pub fn spawn(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        size: UVec2,
        owner: Entity,
    ) -> Self {
        Self::spawn_sampled(commands, images, (size, ImageSampler::nearest()), owner)
    }

    /// Like [`UiCanvas::spawn`], but sampled smoothly, for text that shows
    /// smaller than the canvas draws it.
    pub fn spawn_smooth(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        size: UVec2,
        owner: Entity,
    ) -> Self {
        Self::spawn_sampled(commands, images, (size, ImageSampler::linear()), owner)
    }

    /// Like [`UiCanvas::spawn_smooth`], but drawn at [`PRINT_SCALE`] times
    /// the resolution, so small type on paper stays sharp up close.
    pub fn spawn_print(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        size: UVec2,
        owner: Entity,
    ) -> Self {
        Self::spawn_scaled(
            commands,
            images,
            (size, ImageSampler::linear(), PRINT_SCALE),
            owner,
        )
    }

    fn spawn_sampled(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        (size, sampler): (UVec2, ImageSampler),
        owner: Entity,
    ) -> Self {
        Self::spawn_scaled(commands, images, (size, sampler, 1.0), owner)
    }

    fn spawn_scaled(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        (size, sampler, scale): (UVec2, ImageSampler, f32),
        owner: Entity,
    ) -> Self {
        let texels = (size.as_vec2() * scale).as_uvec2();
        let mut image =
            Image::new_target_texture(texels.x, texels.y, TextureFormat::Rgba8UnormSrgb, None);
        image.sampler = sampler;
        // So lights can follow what it shows.
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let image = images.add(image);
        let camera = commands
            .spawn((
                Camera2d,
                Camera {
                    order: -1,
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                },
                RenderTarget::Image(ImageRenderTarget {
                    handle: image.clone(),
                    scale_factor: scale,
                }),
                RenderLayers::layer(CANVAS_LAYER),
                CanvasOf(owner),
            ))
            .id();
        Self {
            image,
            camera,
            size,
            scale,
            owner,
        }
    }

    /// Makes a UI root draw into this canvas.
    pub fn target(&self) -> impl Bundle {
        (UiTargetCamera(self.camera), CanvasOf(self.owner))
    }
}

/// A mesh showing a [`UiCanvas`] that forwards the mouse into that UI.
///
/// The mesh UVs must span the whole canvas, like [`CrtGlass`] does. The
/// surface entity is also the virtual pointer that drives the canvas UI.
#[derive(Component, Debug)]
#[require(Transform, WantsArrow)]
pub struct ScreenSurface {
    target: NormalizedRenderTarget,
    size: Vec2,
    cursor: Option<Vec2>,
    /// Only active surfaces receive the mouse.
    pub active: bool,
}

impl ScreenSurface {
    pub fn new(canvas: &UiCanvas) -> Self {
        Self {
            // The UI only picks with a pointer on the same target, scale
            // included.
            target: NormalizedRenderTarget::Image(ImageRenderTarget {
                handle: canvas.image.clone(),
                scale_factor: canvas.scale,
            }),
            size: canvas.size.as_vec2(),
            cursor: None,
            active: false,
        }
    }

    fn add_pointer(add: On<Add<ScreenSurface>>, mut commands: Commands) {
        let id = PointerId::Custom(Uuid::from_u64_pair(0x5C12_EE17, add.entity.to_bits()));
        commands.entity(add.entity).insert(id);
    }

    /// Stops clicks on the world from taking focus away from the UI, like
    /// the editor on the computer screen.
    ///
    /// The real mouse clicks the surface mesh whenever the virtual pointer
    /// clicks the UI, and keys or the desk around it can be clicked too.
    /// Without this the click would bubble to the window, which clears
    /// focus.
    fn keep_ui_focus(
        mut acquire: On<AcquireFocus>,
        world: Query<(), (Without<Node>, Without<Window>)>,
    ) {
        if world.contains(acquire.focused_entity) {
            acquire.propagate(false);
        }
    }

    fn drive_pointers(
        app_state: Res<State<AppState>>,
        mut raycast: MeshRayCast,
        window: Single<&Window, With<PrimaryWindow>>,
        cameras: Query<(&Camera, &GlobalTransform, &RenderTarget), With<Camera3d>>,
        mut surfaces: Query<(Entity, &mut ScreenSurface, &PointerId, &mut WantsArrow)>,
        mut window_events: MessageReader<WindowEvent>,
        mut inputs: MessageWriter<PointerInput>,
    ) {
        // Still paused on the frame that resumes, so the click that resumes
        // is dropped as well.
        if *app_state.get() != AppState::Playing {
            window_events.clear();
            return;
        }
        // The nearest surface under the real mouse, if any. Bevy's `RayMap`
        // is only rebuilt after pointer inputs are processed, so it would be
        // a frame late here. We cast from the current cursor instead.
        let settings = MeshRayCastSettings {
            visibility: RayCastVisibility::VisibleInView,
            filter: &|entity| {
                surfaces
                    .get(entity)
                    .is_ok_and(|(_, surface, ..)| surface.active)
            },
            early_exit_test: &|_| true,
        };
        let hit = window.cursor_position().and_then(|cursor| {
            cameras.iter().find_map(|(camera, transform, target)| {
                // Only cameras that draw to the window see the cursor.
                if !matches!(target, RenderTarget::Window(WindowRef::Primary)) {
                    return None;
                }
                let ray = camera.viewport_to_world(transform, cursor).ok()?;
                let (entity, hit) = raycast.cast_ray(ray, &settings).first()?;
                Some((*entity, hit.uv?))
            })
        });

        // Read once, because every surface needs to see the releases.
        let actions: Vec<PointerAction> = window_events
            .read()
            .filter_map(Self::pointer_action)
            .collect();

        for (entity, mut surface, &id, mut arrow) in &mut surfaces {
            let hit = hit
                .filter(|(e, _)| *e == entity)
                .map(|(_, uv)| uv * surface.size);
            let last = surface.cursor;
            let Some(position) = hit.or(last) else {
                continue;
            };
            let location = Location {
                target: surface.target.clone(),
                position,
            };

            if hit.is_some() && last != hit {
                inputs.write(PointerInput::new(
                    id,
                    location.clone(),
                    PointerAction::Move {
                        delta: position - last.unwrap_or(position),
                    },
                ));
            }
            for action in &actions {
                // Everything needs the cursor on the screen except releases,
                // so a drag that leaves the screen still ends.
                if hit.is_some() || matches!(action, PointerAction::Release(_)) {
                    inputs.write(PointerInput::new(id, location.clone(), *action));
                }
            }
            surface.cursor = hit;
            arrow.set_if_neq(WantsArrow(hit.is_some()));
        }
    }

    /// Mouse buttons and the wheel, as pointer actions for the canvas.
    fn pointer_action(event: &WindowEvent) -> Option<PointerAction> {
        match event {
            WindowEvent::MouseButtonInput(input) => {
                let button = match input.button {
                    MouseButton::Left => PointerButton::Primary,
                    MouseButton::Middle => PointerButton::Middle,
                    _ => return None,
                };
                Some(match input.state {
                    ButtonState::Pressed => PointerAction::Press(button),
                    ButtonState::Released => PointerAction::Release(button),
                })
            }
            WindowEvent::MouseWheel(wheel) => Some(PointerAction::Scroll {
                unit: wheel.unit,
                x: wheel.x,
                y: wheel.y,
                phase: wheel.phase,
            }),
            _ => None,
        }
    }
}
