mod common;

use common::{SUMMA, TALLY_1_0, build, code, error};
use ld60_calco::{Op, Revision};
use ld60_devices::DeviceKind;
use ld60_dsl::{Flavor, Output};
use ld60_machine::Isa;

/// Every instruction in the assembled image runs on `revision`.
fn assert_supported(output: &Output, revision: Revision) {
    let assembly = revision.isa().assemble(&output.asm).unwrap();
    for line in &assembly.lines {
        let addr = usize::from(line.addr);
        let byte = |offset: usize| {
            assembly
                .image
                .words()
                .get(addr + offset)
                .map_or(0, |&w| w as u8)
        };
        let op = Op::decode(byte(0), byte(1));
        assert!(revision.supports(op), "{op:?} at {addr}\n{}", output.asm);
    }
}

#[test]
fn rev_a_inlines_a_routine_with_one_call() {
    let output = build(
        TALLY_1_0,
        Revision::A,
        DeviceKind::Tally2,
        Flavor::Fixed,
        &[],
    )
    .unwrap();
    assert_eq!(
        code(&output),
        "\
reset:  LDI  A, 0
        ST   M1
        ST   M2
main:   HLT
        IN   KEY
        ST   M3
        CPI  0x0D
        JZ   reset
        CPI  0x0A
        JZ   count
        JMP  main

count:  LD   M2
        INC
        CPI  10
        JC   ones
        LDI  A, 0
        ST   M2
tens:   LD   M1
        INC
        CPI  10
        JC   tstore
        LDI  A, 0
tstore: ST   M1
        JMP  main

ones:   ST   M2
        JMP  main
"
    );
    assert_supported(&output, Revision::A);
}

#[test]
fn rev_a_unrolls_loops_with_a_known_count() {
    let source = "\
firmware \"T\" for tally2
cells tally[2] at M1
entry reset {
    a = 0
    for i = 2 downto 1 as clr {
        tally[i] = a
    }
    wait
}
";
    let output = build(source, Revision::A, DeviceKind::Tally2, Flavor::Fixed, &[]).unwrap();
    assert_eq!(
        code(&output),
        "\
reset:  LDI  A, 0
clr:    ST   M2
        ST   M1
        HLT
"
    );
    assert_supported(&output, Revision::A);
}

#[test]
fn rev_a_names_what_it_cannot_do() {
    for (body, message) in [
        ("ANN = 1", "'OUT p' is not supported by CalCo Rev A"),
        ("a -= 1", "'DEC' is not supported by CalCo Rev A"),
        ("carry = 0", "'CLC' is not supported by CalCo Rev A"),
        (
            "repeat i {\n        wait\n    }",
            "CalCo Rev A cannot run this loop. Set i to a number right before it so it can be \
             unrolled",
        ),
        (
            "a = bcd_add(tally[1], tally[2])",
            "'ADC Mn' is not supported by CalCo Rev A",
        ),
    ] {
        let source = format!(
            "firmware \"T\" for tally2\ncells tally[2] at M1\nentry main {{\n    {body}\n}}\n"
        );
        let diagnostic = error(build(
            &source,
            Revision::A,
            DeviceKind::Tally2,
            Flavor::Fixed,
            &[],
        ));
        assert_eq!(diagnostic.message, message, "{body}");
        assert_eq!(diagnostic.line, 4, "{body}");
    }
}

#[test]
fn rev_a_rejects_the_summa_firmware() {
    let diagnostic = error(build(
        SUMMA,
        Revision::A,
        DeviceKind::Summa6,
        Flavor::Fixed,
        &[],
    ));
    assert!(diagnostic.message.contains("CalCo Rev A"), "{diagnostic}");
}

#[test]
fn rev_a_branches_around_missing_jumps() {
    let source = "\
firmware \"T\" for tally2
entry main {
    wait
    a = KEY
    if a == 0x0A {
        a = 1
    }
    goto main
}
";
    let output = build(source, Revision::A, DeviceKind::Tally2, Flavor::Fixed, &[]).unwrap();
    assert_eq!(
        code(&output),
        "\
main:   HLT
        IN   KEY
        CPI  0x0A
        JZ   mskip
        JMP  mskip2
mskip:  LDI  A, 1
mskip2: JMP  main
"
    );
    assert_supported(&output, Revision::A);
}

#[test]
fn rev_b_and_c_keep_calls_and_loops() {
    for revision in [Revision::B, Revision::C] {
        for flavor in [Flavor::Buggy, Flavor::Fixed] {
            let output = build(SUMMA, revision, DeviceKind::Summa6, flavor, &[]).unwrap();
            assert!(output.asm.contains("CALL apply"));
            assert!(output.asm.contains("DJNZ aloop"));
            assert_supported(&output, revision);
        }
    }
}

#[test]
fn routines_with_several_calls_get_fresh_labels() {
    let source = "\
firmware \"T\" for tally2
cell n at M1
entry main {
    bump()
    bump()
    wait
}
routine bump {
    a = n + 1
    if a >= 10 {
        a = 0
    }
    n = a
}
";
    let output = build(source, Revision::A, DeviceKind::Tally2, Flavor::Fixed, &[]).unwrap();
    assert_eq!(
        code(&output),
        "\
main:   LD   M1
        INC
        CPI  10
        JC   bstore
        LDI  A, 0
bstore: ST   M1
        LD   M1
        INC
        CPI  10
        JC   bstor2
        LDI  A, 0
bstor2: ST   M1
        HLT
"
    );
}
