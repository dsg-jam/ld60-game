use ld60_machine::{Image, Ram, StuckBit};

#[test]
fn stuck_bits_override_writes() {
    let mut ram = Ram::new(16);
    ram.add_stuck_bit(StuckBit {
        cell: 7,
        bit: 0,
        high: true,
    });
    assert_eq!(ram.read(7), 1);
    ram.write(7, 0);
    assert_eq!(ram.read(7), 1);
    ram.write(6, 0);
    assert_eq!(ram.read(6), 0);
}

#[test]
fn fill_respects_stuck_bits() {
    let mut ram = Ram::new(4);
    ram.add_stuck_bit(StuckBit {
        cell: 2,
        bit: 7,
        high: false,
    });
    ram.fill(|| 0xFF);
    assert_eq!(ram.cells(), &[0xFF, 0xFF, 0x7F, 0xFF]);
}

#[test]
fn blank_image_is_erased() {
    let image = Image::blank(4, 11);
    assert_eq!(image.mask(), 0x7FF);
    assert!(image.words().iter().all(|&w| w == 0x7FF));
}
