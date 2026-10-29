use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_devices::DeviceKind;
use ld60_game_audio::{Play, Sound};
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::state::AppState;
use ld60_game_parts::paper::{INK_FADED, PAPER, Paper};
use ld60_game_parts::screen::UiCanvas;
use ld60_game_player::focus::{Clickable, Focus, FocusTarget, Focused, Prompting, View};
use ld60_game_run::salvage::Find;
use ld60_game_run::{Leftover, Rent, Run};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use ld60_locale::Message;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

use super::rent::RentNote;

/// At the front right of the counter, beside the keyboard.
pub(super) const SPOT: Vec2 = Vec2::new(1.0, 0.72);
pub(super) const YAW: f32 = -0.25;
pub(super) const CRATE_SIZE: Vec3 = Vec3::new(0.24, 0.07, 0.17);
/// The order the trader offers new devices in.
const DEVICE_ORDER: [DeviceKind; 3] = [DeviceKind::Score5, DeviceKind::Summa6, DeviceKind::Lock4];
const WALL: f32 = 0.008;
/// The packing straw fills the crate up to this height.
const STRAW_HEIGHT: f32 = 0.045;
const FIND_SIZE: Vec3 = Vec3::new(0.06, 0.012, 0.04);
const TAG_SIZE: Vec3 = Vec3::new(0.09, 0.0004, 0.063);
const TAG_CANVAS: UVec2 = UVec2::new(320, 224);
/// Where each find sits in the crate, from -1 to 1 along x and z. The first
/// find is at the front left, nearest to the player.
const CELLS: [Vec2; MOST] = [
    Vec2::new(-1.0, 1.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(-1.0, -1.0),
    Vec2::new(1.0, -1.0),
];
const FEWEST: usize = 3;
const MOST: usize = 4;
const CRATE_WOOD: Color = Color::srgb_u8(0x8a, 0x6a, 0x44);
const POST_WOOD: Color = Color::srgb_u8(0x6a, 0x4e, 0x30);
const NAIL: Color = Color::srgb_u8(0x5a, 0x5c, 0x5e);
/// Each wall is two slats with a gap between them.
const SLAT_GAP: f32 = 0.007;
/// The square posts in the corners, a little taller than the walls.
const POST: Vec3 = Vec3::new(0.014, 0.074, 0.014);
/// The grips on the short sides.
const GRIP: Vec3 = Vec3::new(0.008, 0.01, 0.06);
const NAIL_HEAD: f32 = 0.003;
const STRAW: Color = Color::srgb_u8(0xc8, 0xb0, 0x6a);
const FINDS: [Color; 4] = [
    Color::srgb_u8(0x4a, 0x4e, 0x52),
    Color::srgb_u8(0x6a, 0x5a, 0x48),
    Color::srgb_u8(0x3a, 0x44, 0x3e),
    Color::srgb_u8(0x5a, 0x50, 0x5a),
];

pub(crate) struct SalvagePlugin;

impl Plugin for SalvagePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(SalvageCrate::deliver)
            .add_observer(SalvageCrate::on_add)
            .add_observer(ForSale::buy_on_press)
            .add_observer(Buy::sell)
            .add_systems(Update, ForSale::arm);
    }
}

/// The scrap trader's crate, left on the counter after the rent is paid.
///
/// It holds a few finds, each with a paper tag that says what the trader
/// thinks it is and what it costs. Click a find to buy it.
#[derive(Component, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[require(Transform, Visibility, Leftover)]
pub(crate) struct SalvageCrate {
    finds: Vec<Find>,
}

/// A find in a [`SalvageCrate`], with its tag.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
struct ForSale(Find);

/// The player wants to buy the [`ForSale`] find.
#[derive(EntityEvent, Debug)]
struct Buy {
    entity: Entity,
}

impl SalvageCrate {
    /// A random few of the finds the shop wants. If there is a device the
    /// shop does not repair yet, the cheapest one is in the crate. No two
    /// finds file the same datasheet.
    fn pack(run: &Run) -> Option<Self> {
        let seed = run.seed() ^ u64::from(run.stats().cycles).rotate_left(40);
        let mut rng = SmallRng::seed_from_u64(seed);
        // New devices come in a fixed order, each easier to learn after the
        // one before.
        let next = DEVICE_ORDER
            .into_iter()
            .find(|device| !run.signs().contains(device));
        let mut finds: Vec<_> = Find::all()
            .filter(|&find| run.wants(find))
            .filter(|&find| !matches!(find, Find::Device(device) if Some(device) != next))
            .collect();
        finds.shuffle(&mut rng);
        if let Some(device) = finds
            .iter()
            .enumerate()
            .filter(|(_, find)| matches!(find, Find::Device(_)))
            .min_by_key(|(_, find)| find.price())
            .map(|(index, _)| index)
        {
            finds.swap(0, device);
        }
        if let Some(deed) = finds.iter().position(|&find| find == Find::Deed) {
            let deed = finds.remove(deed);
            finds.insert(finds.len().min(1), deed);
        }
        let size = rng.random_range(FEWEST..=MOST);
        let mut packed: Vec<Find> = Vec::new();
        for find in finds {
            if packed.len() == size {
                break;
            }
            let datasheet = find.datasheet();
            if datasheet.is_none() || packed.iter().all(|other| other.datasheet() != datasheet) {
                packed.push(find);
            }
        }
        (!packed.is_empty()).then_some(Self { finds: packed })
    }

    /// The trader comes once the landlord got the rent, and takes back the
    /// crate from last time.
    fn deliver(
        add: On<Add<RentNote>>,
        notes: Query<&RentNote>,
        run: Res<Run>,
        crates: Query<Entity, With<SalvageCrate>>,
        mut commands: Commands,
    ) {
        let Ok(note) = notes.get(add.entity) else {
            return;
        };
        if matches!(note.rent(), Rent::Short { .. }) {
            return;
        }
        for old in &crates {
            commands.entity(old).despawn();
        }
        if let Some(salvage) = Self::pack(&run) {
            commands.spawn(salvage.on_counter());
        }
    }

    /// The crate in its place on the counter.
    pub(crate) fn on_counter(self) -> impl Bundle {
        (
            Name::new("salvage crate"),
            self,
            CounterLayout::new().place(SPOT, YAW),
            FocusTarget {
                prompt: "click-crate",
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(Vec3::Y * CRATE_SIZE.y)),
            },
        )
    }

    fn on_add(
        add: On<Add<SalvageCrate>>,
        crates: Query<&SalvageCrate>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
    ) {
        let Ok(salvage) = crates.get(add.entity) else {
            return;
        };
        let finds = &salvage.finds[..salvage.finds.len().min(MOST)];
        let size = TAG_CANVAS * UVec2::new(MOST as u32, 1);
        let canvas = UiCanvas::spawn(&mut commands, &mut images, size, add.entity);
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(PAPER),
                canvas.target(),
            ))
            .with_children(|tags| {
                for &find in finds {
                    tags.spawn(ForSale::tag(find));
                }
            });

        let floor = Block::new(CRATE_SIZE.with_y(WALL));
        let straw =
            Block::new((CRATE_SIZE - 2.0 * WALL).with_y(STRAW_HEIGHT - WALL)).on_top_of(&floor);
        let inside = straw.size.xz();
        commands.entity(add.entity).with_children(|salvage| {
            salvage.spawn((Shape::cuboid(&floor, CRATE_WOOD), Finish::Wood));
            salvage.spawn((Shape::cuboid(&straw, STRAW), Finish::Fabric));
            let slat = (CRATE_SIZE.y - SLAT_GAP) / 2.0;
            for side in [-1.0, 1.0] {
                for low in [0.0, slat + SLAT_GAP] {
                    let long = Block::new(Vec3::new(CRATE_SIZE.x - 2.0 * POST.x, slat, WALL))
                        .translated(Vec3::new(0.0, low, side * (CRATE_SIZE.z - WALL) / 2.0));
                    let short = Block::new(Vec3::new(WALL, slat, CRATE_SIZE.z - 2.0 * POST.z))
                        .translated(Vec3::new(side * (CRATE_SIZE.x - WALL) / 2.0, low, 0.0));
                    for wall in [long, short] {
                        salvage.spawn((Shape::cuboid(&wall, CRATE_WOOD), Finish::Wood));
                    }
                    // Nailed to the posts at both ends.
                    for end in [-1.0, 1.0] {
                        let nail = Block::new(Vec3::new(NAIL_HEAD, NAIL_HEAD, 0.001)).translated(
                            Vec3::new(
                                end * (CRATE_SIZE.x - POST.x) / 2.0,
                                low + slat / 2.0,
                                side * CRATE_SIZE.z / 2.0,
                            ),
                        );
                        salvage.spawn((Shape::cuboid(&nail, NAIL), Finish::Metal));
                    }
                }
                for end in [-1.0, 1.0] {
                    let post = Block::new(POST).translated(Vec3::new(
                        side * (CRATE_SIZE.x - POST.x) / 2.0,
                        0.0,
                        end * (CRATE_SIZE.z - POST.z) / 2.0,
                    ));
                    salvage.spawn((Shape::cuboid(&post, POST_WOOD), Finish::Wood));
                }
                let grip = Block::new(GRIP).translated(Vec3::new(
                    side * (CRATE_SIZE.x + GRIP.x) / 2.0,
                    CRATE_SIZE.y * 0.6,
                    0.0,
                ));
                salvage.spawn((Shape::cuboid(&grip, POST_WOOD), Finish::Wood));
            }
            // Loose tufts, so the straw is not flat.
            for (x, z, height) in [
                (-0.4, 0.0, 0.006),
                (0.0, 0.7, 0.004),
                (0.45, -0.1, 0.007),
                (0.0, -0.7, 0.005),
            ] {
                let tuft = Block::new(Vec3::new(0.05, height, 0.035)).translated(Vec3::new(
                    x * inside.x / 2.0,
                    straw.max().y,
                    z * inside.y / 2.0,
                ));
                salvage.spawn((Shape::cuboid(&tuft, STRAW), Finish::Fabric));
            }
            for (index, &find) in finds.iter().enumerate() {
                let cell = CELLS[index] * inside / 4.0;
                let lump = Block::new(FIND_SIZE.with_y(FIND_SIZE.y * (1.0 + index as f32 * 0.3)))
                    .translated(Vec3::new(cell.x, straw.max().y, cell.y));
                let tag = Block::new(TAG_SIZE).resting_on(&lump, Vec2::ZERO);
                let region = Rect::new(
                    index as f32 / MOST as f32,
                    0.0,
                    (index + 1) as f32 / MOST as f32,
                    1.0,
                );
                salvage.spawn(ForSale(find)).with_children(|offer| {
                    offer.spawn((
                        Shape::cuboid(&lump, FINDS[index % FINDS.len()]),
                        Finish::Metal,
                    ));
                    offer.spawn((
                        Shape::textured_cuboid(&tag, canvas.image.clone(), region),
                        Finish::Paper,
                    ));
                });
            }
        });
    }
}

impl ForSale {
    fn tag(find: Find) -> impl Bundle {
        (
            Node {
                width: px(TAG_CANVAS.x),
                height: px(TAG_CANVAS.y),
                flex_direction: FlexDirection::Column,
                padding: px(14).all(),
                row_gap: px(6),
                border: px(2).all(),
                ..default()
            },
            BorderColor::all(INK_FADED),
            children![
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_height: px(0),
                        row_gap: px(6),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    children![
                        Paper::heading(Message::new("paper-quote").with("text", find.guess())),
                        Paper::faded(find.note()),
                    ],
                ),
                (
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                    children![Paper::title(Self::price(find))],
                ),
            ],
        )
    }

    fn price(find: Find) -> Message {
        Message::new("receipt-amount").with("amount", find.price())
    }

    /// While the crate is open, each find can be bought once the till holds
    /// enough for it.
    fn arm(
        run: Res<Run>,
        focused: Res<Focused>,
        offers: Query<(Entity, &ForSale, &ChildOf, Prompting)>,
        mut commands: Commands,
    ) {
        for (entity, offer, parent, now) in &offers {
            let wanted = (focused.get() == Some(parent.parent()))
                .then_some(("click-buy", run.till() >= offer.0.price()));
            Clickable::show(&mut commands, entity, now, wanted);
        }
    }

    /// A click on a find in the open crate buys it.
    fn buy_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        offers: Query<&ChildOf, With<ForSale>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Paper {
            return;
        }
        let Ok(parent) = offers.get(press.entity) else {
            return;
        };
        if focused.get() != Some(parent.parent()) {
            return;
        }
        press.propagate(false);
        commands.trigger(Buy {
            entity: press.entity,
        });
    }
}

impl Buy {
    /// Takes the money from the till and the find out of the crate, if the
    /// till holds enough. The empty crate goes away. Buying the deed wins
    /// the run.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn sell(
        buy: On<Buy>,
        mut focused: ResMut<Focused>,
        mut focus: ResMut<NextState<Focus>>,
        offers: Query<(&ForSale, &ChildOf)>,
        mut crates: Query<&mut SalvageCrate>,
        mut run: ResMut<Run>,
        mut app_state: ResMut<NextState<AppState>>,
        mut commands: Commands,
    ) {
        let Ok((&ForSale(find), parent)) = offers.get(buy.entity) else {
            return;
        };
        if !run.buy(find) {
            commands.trigger(Play(Sound::Thunk));
            return;
        }
        commands.trigger(Play(Sound::Chime));
        commands.entity(buy.entity).despawn();
        if find == Find::Deed {
            app_state.set(AppState::Owned);
        }
        let salvage = parent.parent();
        let Ok(mut salvage_crate) = crates.get_mut(salvage) else {
            return;
        };
        salvage_crate.finds.retain(|&other| other != find);
        if salvage_crate.finds.is_empty() {
            if focused.get() == Some(salvage) {
                focused.leave(&mut focus);
            }
            commands.entity(salvage).despawn();
        }
    }
}

#[cfg(test)]
impl SalvageCrate {
    pub(crate) fn with_finds(finds: Vec<Find>) -> Self {
        Self { finds }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ld60_devices::DeviceKind;
    use ld60_game_player::focus::FocusOn;
    use ld60_game_run::ComputerTool;
    use ld60_game_run::chore::PlanetSet;
    use ld60_locale::{Locale, Localized};

    use super::*;

    fn paid_rent(run: &mut Run) {
        run.add_till(run.rent());
        assert!(matches!(run.pay_rent(), Rent::Paid { .. }));
    }

    #[test]
    fn every_tag_formats() {
        let locale = Locale::on_disk();
        for find in Find::all() {
            locale.assert_formats(Localized::spawned_by(|tags| {
                tags.spawn(ForSale::tag(find));
            }));
        }
        locale.assert_formats(["click-crate", "click-buy"].map(Message::new));
    }

    #[test]
    fn the_crate_holds_three_or_four_wanted_finds() {
        let mut run = Run::default().next();
        for _ in 0..30 {
            paid_rent(&mut run);
            let salvage = SalvageCrate::pack(&run).unwrap();
            assert!((FEWEST..=MOST).contains(&salvage.finds.len()));
            assert!(salvage.finds.iter().all(|&find| run.wants(find)));
            assert!(
                matches!(salvage.finds[0], Find::Device(device) if !run.signs().contains(&device)),
                "{:?}",
                salvage.finds
            );
            let unique: HashSet<_> = salvage.finds.iter().collect();
            assert_eq!(unique.len(), salvage.finds.len());
        }
    }

    #[test]
    fn new_devices_come_in_order() {
        let mut run = Run::default().next();
        for _ in 0..10 {
            paid_rent(&mut run);
            let salvage = SalvageCrate::pack(&run).unwrap();
            assert_eq!(salvage.finds[0], Find::Device(DeviceKind::Score5));
            let devices = salvage
                .finds
                .iter()
                .filter(|find| matches!(find, Find::Device(_)))
                .count();
            assert_eq!(devices, 1);
        }
    }

    #[test]
    fn every_crate_holds_the_deed_once_the_landlord_sells() {
        let mut run = Run::default().next();
        for _ in 0..10 {
            paid_rent(&mut run);
            let salvage = SalvageCrate::pack(&run).unwrap();
            assert_eq!(
                salvage.finds.contains(&Find::Deed),
                run.wants(Find::Deed),
                "{:?}",
                salvage.finds
            );
        }
        assert!(run.wants(Find::Deed));
    }

    #[test]
    fn buying_the_deed_wins_the_run() {
        let (mut world, salvage, _) = crate_world(Find::Deed.price());
        world.resource_mut::<Run>().offer_deed();
        let deed = world.spawn((ForSale(Find::Deed), ChildOf(salvage))).id();
        world.trigger(Buy { entity: deed });
        world.flush();
        assert!(world.resource::<Run>().owned());
        assert!(matches!(
            *world.resource::<NextState<AppState>>(),
            NextState::Pending(AppState::Owned)
        ));
    }

    #[test]
    fn no_two_finds_bring_the_same_datasheet() {
        let mut run = Run::default().next();
        for _ in 0..30 {
            paid_rent(&mut run);
            let salvage = SalvageCrate::pack(&run).unwrap();
            let datasheets: Vec<_> = salvage
                .finds
                .iter()
                .filter_map(|find| find.datasheet())
                .collect();
            let unique: HashSet<_> = datasheets.iter().collect();
            assert_eq!(unique.len(), datasheets.len(), "{:?}", salvage.finds);
        }
    }

    #[test]
    fn the_stock_changes_from_one_rent_to_the_next() {
        let mut run = Run::default().next();
        let stocks: Vec<_> = (0..10)
            .map(|_| {
                paid_rent(&mut run);
                SalvageCrate::pack(&run).unwrap()
            })
            .collect();
        assert!(stocks.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn breakpoints_need_the_debugger() {
        let mut run = Run::default();
        let breakpoints = Find::Tool(ComputerTool::Breakpoints);
        assert!(!run.wants(breakpoints));
        run.add_till(100);
        assert!(run.buy(Find::Tool(ComputerTool::Debugger)));
        assert!(run.wants(breakpoints));
    }

    #[test]
    fn a_sold_out_trader_brings_no_crate() {
        let mut run = Run::default();
        run.add_till(1000);
        for find in Find::all().collect::<Vec<_>>() {
            if run.wants(find) {
                assert!(run.buy(find), "{find:?}");
            }
        }
        assert_eq!(SalvageCrate::pack(&run), None);
    }

    fn crate_world(till: u32) -> (World, Entity, Entity) {
        let mut world = World::new();
        let mut run = Run::default();
        run.add_till(till);
        world.insert_resource(run);
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.init_resource::<NextState<AppState>>();
        world.add_observer(Buy::sell);
        world.add_observer(FocusOn::apply);
        let find = Find::Device(DeviceKind::Summa6);
        let salvage = world
            .spawn((
                SalvageCrate { finds: vec![find] },
                FocusTarget {
                    prompt: "click-crate",
                    focus: Focus::Paper,
                    view: View::World(Transform::IDENTITY),
                },
            ))
            .id();
        let offer = world.spawn((ForSale(find), ChildOf(salvage))).id();
        world.trigger(FocusOn { entity: salvage });
        (world, salvage, offer)
    }

    #[test]
    fn buying_takes_the_money_and_files_the_find() {
        let price = Find::Device(DeviceKind::Summa6).price();
        let (mut world, salvage, offer) = crate_world(price + 1);
        world.trigger(Buy { entity: offer });
        world.flush();
        let run = world.resource::<Run>();
        assert_eq!(run.till(), 1);
        assert!(run.signs().contains(&DeviceKind::Summa6));
        assert_eq!(run.stats().spent, price);
        assert!(world.get_entity(offer).is_err());
        assert!(world.get_entity(salvage).is_err());
        assert_eq!(world.resource::<Focused>().get(), None);
    }

    #[test]
    fn a_short_till_buys_nothing() {
        let price = Find::Device(DeviceKind::Summa6).price();
        let (mut world, salvage, offer) = crate_world(price - 1);
        world.trigger(Buy { entity: offer });
        world.flush();
        let run = world.resource::<Run>();
        assert_eq!(run.till(), price - 1);
        assert!(!run.signs().contains(&DeviceKind::Summa6));
        assert!(world.get_entity(offer).is_ok());
        assert_eq!(world.resource::<Focused>().get(), Some(salvage));
    }

    #[test]
    fn paid_rent_brings_a_new_crate() {
        let mut world = World::new();
        let mut run = Run::default().next();
        run.add_till(1000);
        world.insert_resource(run);
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.init_resource::<Assets<Image>>();
        world.add_observer(RentNote::collect);
        world.add_observer(SalvageCrate::deliver);
        world.add_observer(SalvageCrate::on_add);
        world.add_observer(FocusOn::apply);
        let crates = |world: &mut World| {
            world
                .query_filtered::<Entity, With<SalvageCrate>>()
                .iter(world)
                .collect::<Vec<_>>()
        };
        world.trigger(PlanetSet);
        world.flush();
        let first = crates(&mut world);
        assert_eq!(first.len(), 1);
        let offers = world.query::<&ForSale>().iter(&world).count();
        assert!((FEWEST..=MOST).contains(&offers));
        world.trigger(PlanetSet);
        world.flush();
        let second = crates(&mut world);
        assert_eq!(second.len(), 1);
        assert_ne!(first, second);
    }

    #[test]
    fn short_rent_brings_no_crate() {
        let mut world = World::new();
        world.insert_resource(Run::default().next());
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(RentNote::collect);
        world.add_observer(SalvageCrate::deliver);
        world.add_observer(FocusOn::apply);
        world.trigger(PlanetSet);
        world.flush();
        assert_eq!(world.query::<&SalvageCrate>().iter(&world).count(), 0);
    }
}
