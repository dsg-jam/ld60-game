use ld60_calco::{Calco, ChipInfo, DateCode, Revision};
use ld60_devices::{AnyDevice, ChipSpec, Device, DeviceKind, MAX_STEPS, Rom, RomKind, RunError};
use ld60_machine::Isa;

const CHIP: ChipSpec = ChipSpec::Calco(ChipInfo {
    revision: Revision::A,
    date_code: DateCode::new(76, 30),
});

fn device(source: &str) -> AnyDevice {
    let image = Calco.assemble(source).unwrap().image;
    AnyDevice::new(DeviceKind::Summa6, CHIP, Rom::new(RomKind::Eprom, image)).unwrap()
}

#[test]
fn display_shows_ram_cells_and_annunciators() {
    let mut d = device(
        "
        LDI  A, 4
        ST   M5
        LDI  A, 2
        ST   M6
        LDI  B, 4
        LDI  A, 0
clr:    ST   [B]
        DJNZ clr
        LDI  A, 1
        OUT  ANN
        HLT
        ",
    );
    d.power_on(7);
    d.run_until_halt(MAX_STEPS).unwrap();
    assert_eq!(d.readout().to_string(), "-000042");
}

#[test]
fn key_press_wakes_halt_and_reads_once() {
    let mut d = device("main: HLT\nIN KEY\nST M6\nIN KEY\nST M5\nJMP main");
    d.power_on(1);
    d.run_until_halt(MAX_STEPS).unwrap();
    d.press_key('7').unwrap();
    d.run_until_halt(MAX_STEPS).unwrap();
    let ram = d.cpu().ram();
    assert_eq!((ram.read(6), ram.read(5)), (7, 0xFF));
}

#[test]
fn power_on_fills_ram_with_garbage() {
    let mut d = device("HLT");
    d.power_on(1);
    let first = d.cpu().ram().cells().to_vec();
    d.power_on(2);
    assert_ne!(first, d.cpu().ram().cells());
}

#[test]
fn reports_endless_loops_and_unknown_keys() {
    let mut d = device("loop: JMP loop");
    d.power_on(0);
    assert_eq!(
        d.run_until_halt(1000),
        Err(RunError::Timeout { steps: 1000 })
    );
    assert_eq!(d.press_key('x'), Err(RunError::UnknownKey('x')));
}
