mod font;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    light::NotShadowCaster,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use self::font::GLYPHS;
use crate::geometry::block::Block;
use crate::geometry::frame::Frame;
use crate::geometry::surface::{Finish, Form, SurfaceMaterial};

/// Glyph rows. Matches `dot_matrix.wesl`.
const ROWS: u32 = 7;
/// Empty dots around the matrix. Matches `dot_matrix.wesl`.
const MARGIN: u32 = 1;
/// Most glyphs of the font leave their last column blank as the gap to the
/// next one. A few, like `#` and `+`, touch their neighbors.
const GLYPH_WIDTH: usize = 5;
/// Seconds a page that fits shows.
const HOLD: f32 = 3.0;
/// Seconds the start of a long page shows before it scrolls, and its end
/// after.
const LEAD: f32 = 1.5;
/// Columns a long page scrolls by per second.
const SPEED: f32 = 12.0;
/// Vectors of text in the shader, with 16 one-byte columns each. Matches
/// `dot_matrix.wesl`.
const TEXT_VECS: usize = 32;
const MAX_COLUMNS: usize = TEXT_VECS * 16;
/// Width of the plastic frame around the screen.
const BEZEL: f32 = 0.003;
/// How deep the module reaches back into whatever it is plugged into.
const DEPTH: f32 = 0.01;
/// How far the clear cover sits behind the front of the frame.
const COVER_RECESS: f32 = 0.001;
/// How far the layer of dots sits behind the cover.
const COVER_GAP: f32 = 0.0015;
/// Dark dots cast a faint shadow onto the reflector this far behind them.
const SHADOW_DEPTH: f32 = 0.0006;
const FRAME: Color = Color::srgb_u8(0x22, 0x1e, 0x1a);
const COVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);

pub(crate) struct DotMatrixPlugin;

impl Plugin for DotMatrixPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "dot_matrix.wesl");
        app.add_plugins(MaterialPlugin::<DotMatrixMaterial>::default())
            .add_systems(PostUpdate, DotMatrix::refresh)
            .add_observer(DotMatrix::on_add);
    }
}

/// A backlit display of dots that shows one line of text at a time, like
/// the displays of radios: it goes through its pages in turn. A page that
/// fits shows whole. A longer one shows its start, scrolls to its end and
/// pauses there.
///
/// It is a module of a plastic frame, a clear cover and the layer of dots
/// behind it. The front of the frame faces +Z and is centered on the entity,
/// and the module reaches back from there, to plug into a hole.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct DotMatrix {
    pub(crate) lit: bool,
    /// Size of a dot, with the gap to the next.
    pitch: f32,
    /// Printable ASCII. Anything else shows as a blank.
    pages: Vec<String>,
    /// Pages that come on once the current one is done.
    next: Option<Vec<String>>,
    page: usize,
    /// When the page came on, `None` until the material shows it.
    since: Option<f32>,
    /// Columns of the page, as rendered.
    length: u32,
    columns: u32,
    material: Handle<DotMatrixMaterial>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct DotMatrixMaterial {
    #[uniform(0)]
    params: DotMatrixParams,
}

/// Matches `DotMatrix` in `dot_matrix.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct DotMatrixParams {
    lit: f32,
    /// Visible columns.
    columns: u32,
    offset: u32,
    length: u32,
    /// How far behind the dots their shadows fall, in dots.
    shadow_depth: f32,
    text: [UVec4; TEXT_VECS],
}

impl DotMatrix {
    /// A display `columns` dots wide, with dots of `pitch` meters.
    pub(crate) fn new(columns: u32, pitch: f32) -> Self {
        Self {
            lit: true,
            pitch,
            pages: Vec::new(),
            next: None,
            page: 0,
            since: None,
            length: columns,
            columns,
            material: Handle::default(),
        }
    }

    /// Width and height of the module, for the hole it plugs into.
    pub(crate) fn outer_size(&self) -> Vec2 {
        self.screen_size() + 2.0 * BEZEL
    }

    /// The dots with the margin around them.
    fn screen_size(&self) -> Vec2 {
        Vec2::new(
            (self.columns + 2 * MARGIN) as f32,
            (ROWS + 2 * MARGIN) as f32,
        ) * self.pitch
    }

    fn on_add(
        add: On<Add<DotMatrix>>,
        mut screens: Query<&mut DotMatrix>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
        mut dot_materials: ResMut<Assets<DotMatrixMaterial>>,
    ) {
        let Ok(mut screen) = screens.get_mut(add.entity) else {
            return;
        };
        let outer = screen.outer_size();
        let size = screen.screen_size();
        let dots_z = -COVER_RECESS - COVER_GAP;
        let frame = Frame {
            outer: Block::between((-outer / 2.0).extend(-DEPTH), (outer / 2.0).extend(0.0)),
            holes: &[Block::between(
                (-size / 2.0).extend(dots_z),
                (size / 2.0).extend(0.0),
            )],
        };
        screen.material = dot_materials.add(DotMatrixMaterial {
            params: DotMatrixParams {
                lit: 0.0,
                columns: screen.columns,
                offset: 0,
                length: 1,
                shadow_depth: SHADOW_DEPTH / screen.pitch,
                text: [UVec4::ZERO; TEXT_VECS],
            },
        });
        let dots = screen.material.clone();
        commands.entity(add.entity).with_children(|module| {
            module.spawn((
                Mesh3d(meshes.add(frame.mesh())),
                MeshMaterial3d(surfaces.add(Finish::Plastic.material(FRAME, Form::Freeform, 0.0))),
            ));
            module.spawn((
                Mesh3d(meshes.add(Rectangle::from_size(size))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: COVER,
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.08,
                    reflectance: 0.5,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.0, -COVER_RECESS),
                NotShadowCaster,
            ));
            module.spawn((
                Mesh3d(meshes.add(Rectangle::from_size(size))),
                MeshMaterial3d(dots),
                Transform::from_xyz(0.0, 0.0, dots_z),
                NotShadowCaster,
            ));
        });
    }

    /// Shows `pages` in turn, once the current page is done.
    pub(crate) fn set_pages(&mut self, pages: &[&str]) {
        let pages: Vec<String> = pages.iter().map(|&page| page.to_owned()).collect();
        if self.pages.is_empty() {
            self.pages = pages;
            self.since = None;
        } else if self.next.as_ref().unwrap_or(&self.pages) != &pages {
            self.next = Some(pages);
        }
    }

    /// Seconds the current page shows, and its first visible column at
    /// `elapsed` seconds.
    fn timing(&self, elapsed: f32) -> (f32, u32) {
        let overflow = self.length.saturating_sub(self.columns);
        if overflow == 0 {
            return (HOLD, 0);
        }
        let scroll = overflow as f32 / SPEED;
        let offset = ((elapsed - LEAD) * SPEED).clamp(0.0, overflow as f32) as u32;
        (LEAD + scroll + LEAD, offset)
    }

    fn refresh(
        time: Res<Time>,
        mut screens: Query<&mut DotMatrix>,
        mut materials: ResMut<Assets<DotMatrixMaterial>>,
    ) {
        let now = time.elapsed_secs();
        for mut screen in &mut screens {
            // Only the display itself reads its state, so changes to it
            // don't need detecting.
            let screen = screen.bypass_change_detection();
            if let Some(since) = screen.since
                && now - since >= screen.timing(now - since).0
            {
                // On to the next page, or to new pages if there are any.
                match screen.next.take() {
                    Some(pages) => {
                        screen.pages = pages;
                        screen.page = 0;
                    }
                    None => screen.page = (screen.page + 1) % screen.pages.len().max(1),
                }
                screen.since = None;
            }
            if screen.since.is_none()
                && let Some(mut material) = materials.get_mut(&screen.material)
            {
                let text = screen.pages.get(screen.page).map_or("", String::as_str);
                let (dots, length) = Self::dots(text, screen.columns as usize);
                material.params.text = dots;
                material.params.length = length;
                screen.length = length;
                screen.since = Some(now);
            }
            let Some(params) = materials.get(&screen.material).map(|m| &m.params) else {
                continue;
            };
            let lit = if screen.lit { 1.0 } else { 0.0 };
            let offset = screen.timing(now - screen.since.unwrap_or(now)).1;
            // Only touch the material when it changes, so it isn't sent
            // to the GPU every frame.
            if (params.lit != lit || params.offset != offset)
                && let Some(mut material) = materials.get_mut(&screen.material)
            {
                material.params.lit = lit;
                material.params.offset = offset;
            }
        }
    }

    /// The dot columns that spell `text`, centered in `columns` if it fits,
    /// and how many there are.
    fn dots(text: &str, columns: usize) -> ([UVec4; TEXT_VECS], u32) {
        let width = (text.chars().count() * GLYPH_WIDTH).min(MAX_COLUMNS);
        let start = columns.saturating_sub(width) / 2;
        let mut bytes = [0u8; MAX_COLUMNS];
        for (index, c) in text.chars().enumerate() {
            let at = start + index * GLYPH_WIDTH;
            if at + GLYPH_WIDTH > MAX_COLUMNS {
                break;
            }
            let glyph = GLYPHS
                .get((c as usize).wrapping_sub(' ' as usize))
                .unwrap_or(&GLYPHS[0]);
            for x in 0..GLYPH_WIDTH {
                bytes[at + x] = glyph
                    .iter()
                    .enumerate()
                    .map(|(row, bits)| ((bits >> (GLYPH_WIDTH - 1 - x)) & 1) << row)
                    .fold(0, |column, bit| column | bit);
            }
        }
        let mut words = [UVec4::ZERO; TEXT_VECS];
        for (word, chunk) in bytes.as_chunks::<4>().0.iter().enumerate() {
            words[word / 4][word % 4] = u32::from_le_bytes(*chunk);
        }
        (words, (start + width).max(columns) as u32)
    }
}

impl DotMatrixMaterial {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("dot_matrix.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl Material for DotMatrixMaterial {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_centered() {
        let (dots, length) = DotMatrix::dots("I", 15);
        assert_eq!(length, 15);
        // "I" sits in columns 5 to 9, with its stem in column 7.
        let byte =
            |column: usize| (dots[column / 16][(column / 4) % 4] >> ((column % 4) * 8)) & 0xff;
        assert_ne!(byte(7), 0);
        assert_eq!(byte(0), 0);
    }

    #[test]
    fn long_pages_pause_at_both_ends() {
        let mut screen = DotMatrix::new(60, 0.001);
        screen.length = 72;
        let (duration, start) = screen.timing(0.0);
        assert_eq!(start, 0);
        assert_eq!(screen.timing(duration - 0.1).1, 12);
        screen.length = 40;
        assert_eq!(screen.timing(1.0), (HOLD, 0));
    }
}
