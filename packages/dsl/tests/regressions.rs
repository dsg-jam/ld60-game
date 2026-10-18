mod common;

use common::{build, code, error};
use ld60_calco::Revision;
use ld60_devices::DeviceKind;
use ld60_dsl::{Diagnostic, Flavor, Output};

fn tally(revision: Revision, body: &str) -> Result<Output, Vec<Diagnostic>> {
    let source = format!("firmware \"T\" for tally2\ncell n at M1\n{body}");
    build(&source, revision, DeviceKind::Tally2, Flavor::Fixed, &[])
}

#[test]
fn asm_labels_with_spaces_before_the_colon() {
    for body in [
        "entry main {\n    asm {\n        ab\u{a0}: NOP\n    }\n}\n",
        "entry main {\n    asm {\n        ab : NOP\n    }\n}\n",
    ] {
        let output = tally(Revision::B, body).unwrap();
        assert_eq!(code(&output), "main:\nab:     NOP\n", "{body:?}");
    }
}

#[test]
fn generated_labels_avoid_asm_labels() {
    let body = "\
entry main {
    if a == 1 {
        a = 2
    }
    asm {
        mskip: NOP
    }
    n = a
}
";
    let output = tally(Revision::B, body).unwrap();
    assert!(output.asm.contains("JNZ  mskip2\n"), "{}", output.asm);
}

#[test]
fn loops_that_jump_to_their_own_label_do_not_unroll() {
    let source = "\
firmware \"T\" for tally2
cells arr[3] at M0
entry main {
    a = 7
    arr[3] = a
    for i = 3 downto 1 as lp {
        a = arr[i]
        a += 1
        arr[i] = a
        if a < 3 goto lp
    }
    wait
}
";
    let diagnostic = error(build(
        source,
        Revision::A,
        DeviceKind::Tally2,
        Flavor::Fixed,
        &[],
    ));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (6, "a loop that jumps to its own label cannot be unrolled")
    );
}

#[test]
fn unrolled_loops_keep_single_use_routines() {
    let body = "\
entry main {
    for i = 3 downto 1 {
        s()
    }
    n = a
    wait
}
routine s {
    a += 1
}
";
    let output = tally(Revision::A, body).unwrap();
    assert_eq!(
        code(&output),
        "main:\ns:      INC\n        INC\n        INC\n        ST   M1\n        HLT\n"
    );
}

#[test]
fn unrolled_loops_keep_the_counter_comment() {
    let body = "\
entry main {
    i = 3 ; three passes
    repeat i {
        a += 1
    } ; loop
    for i = 2 downto 1 {
        a += 1
    } ; second
    n = a
    wait
}
";
    let output = tally(Revision::A, body).unwrap();
    assert_eq!(
        code(&output),
        "\
main:
        ; loop
        INC                 ; three passes
        INC
        INC
        INC                 ; second
        INC
        ST   M1
        HLT
"
    );
}

#[test]
fn unrolled_loops_with_ifs_get_fresh_labels() {
    let source = "\
firmware \"T\" for tally2
cells arr[3] at M0
entry main {
    for i = 3 downto 1 as lp {
        a = arr[i]
        if a == 0 {
            a = 5
        }
        arr[i] = a
    }
    wait
}
";
    let output = build(source, Revision::A, DeviceKind::Tally2, Flavor::Fixed, &[]).unwrap();
    assert!(output.asm.contains("mstor3: ST   M0"), "{}", output.asm);
}

#[test]
fn asm_labels_block_copies() {
    let looped = "\
entry main {
    for i = 2 downto 1 {
        asm {
            here: NOP
        }
    }
    wait
}
";
    let diagnostic = error(tally(Revision::A, looped));
    assert_eq!(
        diagnostic.message,
        "loops with asm labels inside cannot be unrolled"
    );
    let called = "\
entry main {
    s()
    s()
    wait
}
routine s {
    asm {
        here: NOP
    }
}
";
    let diagnostic = error(tally(Revision::A, called));
    assert_eq!(
        diagnostic.message,
        "s has asm labels and more than one call, so it cannot be copied"
    );
}
