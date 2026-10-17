use devices::{ProgramError, Rom, RomKind, can_program};
use machine::Image;

fn image(words: &[u16]) -> Image {
    let mut image = Image::blank(words.len(), 8);
    for (addr, &w) in words.iter().enumerate() {
        image.set(addr, w);
    }
    image
}

#[test]
fn programming_only_clears_bits() {
    assert!(can_program(0xFF, 0x31));
    assert!(can_program(0x31, 0x30));
    assert!(!can_program(0x05, 0x06));
}

#[test]
fn eprom_rejects_bit_conflicts_until_erased() {
    let mut rom = Rom::new(RomKind::Eprom, image(&[0x31, 0x05]));
    let fixed = image(&[0x31, 0x06]);
    assert_eq!(
        rom.program(&fixed),
        Err(ProgramError::BitConflict { addr: 1 })
    );
    rom.erase().unwrap();
    rom.program(&fixed).unwrap();
    assert_eq!(rom.image(), &fixed);
}

#[test]
fn faded_bits_can_be_burned_back() {
    let listing = image(&[0xB0, 0x40]);
    let mut rom = Rom::new(RomKind::Eprom, listing.clone());
    rom.fade(1, 0x80);
    assert_eq!(rom.image().get(1), 0xC0);
    rom.program(&listing).unwrap();
}

#[test]
fn mask_rom_cannot_change() {
    let mut rom = Rom::new(RomKind::Mask, image(&[0x00]));
    assert_eq!(rom.program(&image(&[0x00])), Err(ProgramError::MaskRom));
    assert_eq!(rom.erase(), Err(ProgramError::MaskRom));
}
