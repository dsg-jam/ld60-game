use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

use super::Emulator;

/// The debug cable of the computer is plugged into the device. Without it the
/// computer cannot flash or debug the device.
#[derive(Component, Debug)]
#[component(on_remove = Connected::on_remove)]
pub(crate) struct Connected;

impl Connected {
    /// Pulling the plug lets a held chip run on and forgets the breakpoints.
    fn on_remove(mut world: DeferredWorld, context: HookContext) {
        if let Some(mut emulator) = world.get_mut::<Emulator>(context.entity) {
            emulator.set_breakpoints(Vec::new());
            if emulator.held() {
                emulator.release();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_levels::{device, initial_rom};

    use super::*;
    use crate::workshop::levels::Levels;

    #[test]
    fn pulling_the_plug_lets_the_chip_run() {
        let levels = Levels::on_disk();
        let source = &levels.levels()[0];
        let level = source.load().unwrap();
        let rom = initial_rom(&level, &source.buggy).unwrap();
        let mut emulator = Emulator::power_on(device(&level, rom).unwrap());
        emulator.hold();
        emulator.set_breakpoints(vec![1]);
        let mut world = World::new();
        let device = world.spawn((emulator, Connected)).id();
        world.entity_mut(device).remove::<Connected>();
        let emulator = world.get::<Emulator>(device).unwrap();
        assert!(!emulator.held());
        assert!(emulator.breakpoints().is_empty());
    }
}
