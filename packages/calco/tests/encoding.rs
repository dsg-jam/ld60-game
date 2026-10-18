mod common;

use ld60_calco::{Op, Revision};
use ld60_machine::{Image, Isa};

#[test]
fn every_opcode_encodes_back_to_itself() {
    for first in 0..=u8::MAX {
        let op = Op::decode(first, 0x5A);
        let expected = if op.size() == 2 {
            vec![first, 0x5A]
        } else {
            vec![first]
        };
        assert_eq!(op.encode(), expected, "{op:?}");
    }
}

#[test]
fn disassembly_reassembles_to_the_same_image() {
    let mut image = Image::blank(ld60_calco::ROM_LEN, 8);
    for addr in 0..image.len() {
        image.set(addr, addr as u16);
    }
    for revision in Revision::ALL {
        let isa = revision.isa();
        let source: Vec<String> = isa
            .disassemble(&image)
            .into_iter()
            .map(|line| line.text)
            .collect();
        let reassembled = isa.assemble(&source.join("\n")).map(|asm| asm.image);
        assert_eq!(reassembled, Ok(image.clone()), "Rev {revision}");
    }
}

#[test]
fn isa_reference_example_matches() {
    let image = common::assemble(
        "
        .org 0x40
add:    LDI  B, 6
        CLC
loop:   LD   [B+8]
        ADC  [B]
        DA
        ST   [B+8]
        DJNZ loop
        RET
        ",
    );
    let bytes: Vec<u16> = image.words()[0x40..0x4A].to_vec();
    assert_eq!(
        bytes,
        [0x31, 0x06, 0x04, 0x98, 0xC0, 0x08, 0xA8, 0xF1, 0x43, 0x02]
    );
}

#[test]
fn assembler_rejects_invalid_operands() {
    for source in [
        "LD M10",
        "LD [A+1]",
        "LD [B+16]",
        "IN ANN",
        "LDI C, 1",
        "JMP 256",
    ] {
        assert!(Revision::B.isa().assemble(source).is_err(), "{source}");
    }
}
