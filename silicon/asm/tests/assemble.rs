use ld60_asm::{Encoder, Instruction, Labels, assemble};
use ld60_machine::AsmErrorKind;

struct Toy;

impl Encoder for Toy {
    fn word_bits(&self) -> u8 {
        8
    }

    fn rom_len(&self) -> usize {
        16
    }

    fn size(&self, instruction: &Instruction) -> Result<usize, AsmErrorKind> {
        match instruction.mnemonic.as_str() {
            "NOP" => Ok(1),
            "JMP" => Ok(2),
            other => Err(AsmErrorKind::UnknownMnemonic {
                mnemonic: other.to_string(),
            }),
        }
    }

    fn encode(&self, instruction: &Instruction, labels: &Labels) -> Result<Vec<u16>, AsmErrorKind> {
        match instruction.mnemonic.as_str() {
            "NOP" => Ok(vec![0x00]),
            "JMP" => {
                let target = labels.value(&instruction.operands[0])?;
                Ok(vec![
                    0xB0,
                    u16::try_from(target).map_err(|_| AsmErrorKind::WordRange { value: target })?,
                ])
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn resolves_forward_labels_and_org() {
    let asm = assemble("start: JMP end\n.org 8\nend: NOP", &Toy).unwrap();
    assert_eq!(&asm.image.words()[..2], &[0xB0, 0x08]);
    assert_eq!(asm.image.get(8), 0x00);
    assert_eq!(asm.labels["end"], 8);
    assert_eq!(asm.line_at(1), Some(1));
    assert_eq!(asm.line_at(8), Some(3));
}

#[test]
fn unused_words_stay_erased() {
    let asm = assemble("NOP", &Toy).unwrap();
    assert!(asm.image.words()[1..].iter().all(|&w| w == 0xFF));
}

#[test]
fn reports_all_errors_with_line_numbers() {
    let errors = assemble("FOO\nJMP nowhere\nx: NOP\nx: NOP", &Toy).unwrap_err();
    let lines: Vec<_> = errors.iter().map(|e| e.line).collect();
    assert_eq!(lines, vec![1, 2, 4]);
    let kinds: Vec<_> = errors.into_iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            AsmErrorKind::UnknownMnemonic {
                mnemonic: "FOO".into()
            },
            AsmErrorKind::UnknownLabel {
                name: "nowhere".into()
            },
            AsmErrorKind::DuplicateLabel { name: "x".into() },
        ]
    );
}

#[test]
fn reports_the_span_of_each_error() {
    let source = "  FOO\nJMP nowhere\nx: NOP\nx: NOP\n.db 1, 300";
    let errors = assemble(source, &Toy).unwrap_err();
    let spans: Vec<_> = errors.iter().map(|e| e.span.clone()).collect();
    assert_eq!(spans, vec![2..5, 4..11, 0..1, 4..10]);
}

#[test]
fn rejects_overlap_and_overflow() {
    assert!(assemble("NOP\n.org 0\nNOP", &Toy).is_err());
    assert!(assemble(".org 15\nJMP 0", &Toy).is_err());
    assert!(assemble(".db 256", &Toy).is_err());
}
