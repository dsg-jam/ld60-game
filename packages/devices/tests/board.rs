use ld60_calco::{ChipInfo, DateCode, Revision};
use ld60_devices::{
    Annunciator, AnyDevice, BoardRev, ChipSpec, Device, DeviceKind, MAX_STEPS, Rom, RomKind,
};

const CHIP: ChipSpec = ChipSpec::Calco(ChipInfo {
    revision: Revision::B,
    date_code: DateCode::new(76, 30),
});

#[test]
fn original_boards_keep_the_first_wiring() {
    let tally = DeviceKind::Tally2.board(BoardRev::A);
    assert_eq!(tally.display_cells(), 1..=2);
    assert_eq!(tally.keys, [('+', 0x0A), ('C', 0x0D)]);

    let summa = DeviceKind::Summa6.board(BoardRev::A);
    assert_eq!(summa.display_cells(), 1..=6);
    let codes: Vec<u8> = "0123456789+-=C"
        .chars()
        .map(|key| summa.key_code(key).unwrap())
        .collect();
    assert_eq!(codes, (0..=0x0D).collect::<Vec<u8>>());
    assert_eq!(summa.ann_bit(Annunciator::Minus), Some(0));
    assert_eq!(summa.ann_bit(Annunciator::Error), Some(1));
}

#[test]
fn every_board_fits_in_ram_and_has_unique_codes() {
    for kind in DeviceKind::ALL {
        for (board, rev) in kind.boards().iter().zip(BoardRev::ALL) {
            assert_eq!(board.rev, rev);
            assert_eq!(board.kind, kind);
            assert!(*board.display_cells().end() < 16);
            let mut codes: Vec<u8> = board.keys.iter().map(|&(_, code)| code).collect();
            codes.sort_unstable();
            codes.dedup();
            assert_eq!(codes.len(), board.keys.len(), "{}", board.name());
        }
    }
}

#[test]
fn board_decides_display_keys_and_annunciators() {
    let source = "main: WAIT\nIN KEY\nST M7\nLDI A, 2\nOUT ANN\nJMP main";
    let image = CHIP.isa().assemble(source).unwrap().image;
    let board = DeviceKind::Summa6.board(BoardRev::B);
    let mut device = AnyDevice::new(board, CHIP, Rom::new(RomKind::Eprom, image)).unwrap();
    device.power_on(1);
    device.cpu_mut().ram_mut().fill(|| 0);
    device.run_until_halt(MAX_STEPS).unwrap();
    device.press_key('+').unwrap();
    device.run_until_halt(MAX_STEPS).unwrap();
    let readout = device.readout();
    assert_eq!(readout.digits, [0, 0, 0, 0, 0, 0x0C]);
    assert!(readout.minus && !readout.error);
    assert_eq!(device.board().name(), "6B");
}

#[test]
fn lock_lights_open_and_shut() {
    let source = "LDI A, 0\nST M6\nST M7\nST M8\nST M9\nLDI A, 5\nOUT ANN\nHLT";
    let image = CHIP.isa().assemble(source).unwrap().image;
    let board = DeviceKind::Lock4.board(BoardRev::B);
    let mut device = AnyDevice::new(board, CHIP, Rom::new(RomKind::Eprom, image)).unwrap();
    device.power_on(1);
    device.run_until_halt(MAX_STEPS).unwrap();
    let readout = device.readout();
    assert!(readout.open && readout.shut && !readout.error);
    assert_eq!(readout.to_string(), "0000 OPEN SHUT");
    assert_eq!(device.board().name(), "4B");
}
