use std::collections::HashMap;

use bevy::prelude::*;
use ld60_game_run::{ComputerTool, Run as ShopRun};

use super::breakpoints::Breakpoints;
use super::listing::Listing;
use super::trace::TraceView;
use ld60_game_device::{Connected, Emulator};

/// Lines the chip ran recently.
const TRACED: Color = Color::srgba(0.1, 0.26, 0.14, 0.4);
/// Lines with a breakpoint.
const BREAK: Color = Color::srgba(0.45, 0.15, 0.1, 0.5);
/// The line the held chip runs next. Amber, to tell it from the selection.
const CURRENT: Color = Color::srgba(0.44, 0.37, 0.06, 0.5);

/// The background of a source line, from the debugging tools.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Shade {
    #[default]
    Plain,
    Traced,
    Break,
    Current,
}

/// The shaded lines of the editor, by 0-based line.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(super) struct Shades(HashMap<usize, Shade>);

impl Shade {
    /// The shades sit on top of the caret and the selection of the editor,
    /// so they let both show through. Over black they are at least 3:1
    /// against every highlight color.
    pub(super) fn color(self) -> Color {
        match self {
            Self::Plain => Color::NONE,
            Self::Traced => TRACED,
            Self::Break => BREAK,
            Self::Current => CURRENT,
        }
    }
}

impl Shades {
    /// Shades the lines of the `trace` addresses, then the `breakpoints`
    /// with code in the chip, then the line at `held`, the address the held
    /// chip runs next. Later shades cover earlier ones.
    pub(super) fn new(
        listing: &Listing,
        trace: impl IntoIterator<Item = u16>,
        breakpoints: impl IntoIterator<Item = usize>,
        held: Option<u16>,
    ) -> Self {
        let mut lines = HashMap::new();
        let traced = trace
            .into_iter()
            .filter_map(|addr| listing.line_at(addr))
            .map(|line| (line, Shade::Traced));
        lines.extend(traced);
        let breaks = breakpoints
            .into_iter()
            .filter(|&line| {
                listing
                    .assembly()
                    .is_some_and(|assembly| Breakpoints::address(assembly, line).is_some())
            })
            .map(|line| (line, Shade::Break));
        lines.extend(breaks);
        if let Some(line) = held.and_then(|addr| listing.line_at(addr)) {
            lines.insert(line, Shade::Current);
        }
        Self(lines)
    }

    pub(super) fn line(&self, index: usize) -> Shade {
        self.0.get(&index).copied().unwrap_or_default()
    }

    pub(super) fn update(
        shop: Res<ShopRun>,
        view: Res<TraceView>,
        listing: Res<Listing>,
        breakpoints: Res<Breakpoints>,
        emulators: Query<&Emulator, With<Connected>>,
        mut shades: ResMut<Shades>,
    ) {
        let emulator = emulators.single().ok();
        let trace = emulator
            .filter(|_| shop.has(ComputerTool::Trace) && view.0)
            .into_iter()
            .flat_map(Emulator::trace);
        let breakpoints = breakpoints
            .lines()
            .filter(|_| shop.has(ComputerTool::Breakpoints));
        let held = emulator
            .filter(|emulator| shop.has(ComputerTool::Debugger) && emulator.held())
            .map(|emulator| emulator.cpu().pc());
        shades.set_if_neq(Self::new(&listing, trace, breakpoints, held));
    }
}

#[cfg(test)]
mod tests {
    use ld60_machine::Isa;

    use super::*;

    #[test]
    fn later_shades_cover_earlier_ones() {
        let isa = ld60_calco::Revision::C.isa();
        let listing = Listing::new(isa.assemble("NOP\nNOP\nNOP\nNOP\n").unwrap());
        let shades = Shades::new(&listing, [0, 1, 2, 9], [1, 2, 4], Some(2));
        let lines: Vec<_> = (0..5).map(|line| shades.line(line)).collect();
        assert_eq!(
            lines,
            [
                Shade::Traced,
                Shade::Break,
                Shade::Current,
                Shade::Plain,
                Shade::Plain
            ]
        );
    }
}
