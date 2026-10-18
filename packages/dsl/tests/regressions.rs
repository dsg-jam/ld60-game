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
    goto main
}
routine s {
    a += 1
}
";
    let output = tally(Revision::A, body).unwrap();
    assert_eq!(
        code(&output),
        "main:\ns:      INC\n        INC\n        INC\n        ST   M1\n        HLT\n        JMP  main\n"
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
fn setting_i_inside_a_loop_is_reported() {
    let body = "\
entry main {
    for i = 3 downto 1 {
        i = 5
        n = a
    }
    wait
}
";
    let diagnostic = error(tally(Revision::B, body));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (5, "i changes inside a loop")
    );
}

#[test]
fn asm_in_a_called_routine_may_change_i() {
    let body = "\
entry main {
    for i = 3 downto 1 {
        p()
    }
    wait
    goto main
}
routine p {
    q()
}
routine q {
    asm {
        LDI B, 7
    }
}
";
    let diagnostic = error(tally(Revision::B, body));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (5, "p changes i inside a loop")
    );
    let direct = "\
entry main {
    for i = 3 downto 1 {
        asm {
            NOP
        }
    }
    wait
}
";
    let diagnostic = error(tally(Revision::B, direct));
    assert_eq!(diagnostic.message, "asm inside a loop may change i");
}

#[test]
fn gotos_cannot_enter_routines() {
    for (body, line, message) in [
        (
            "entry main {\n    s()\n    s()\n    goto s\n}\nroutine s {\n    a += 1\n}\n",
            6,
            "s is a routine. Call it with s()",
        ),
        (
            "entry main {\n    s()\n    goto inner\n}\nroutine s {\n    a += 1\ninner:\n    a += 1\n}\n",
            5,
            "'inner' is inside routine s. Only s can jump to it",
        ),
        (
            "entry main {\n    s()\n    s()\n    goto inner\n}\nroutine s {\n    a += 1\ninner:\n    a += 1\n}\n",
            6,
            "'inner' is inside routine s. Only s can jump to it",
        ),
        (
            "entry main {\n    p()\n    n = a\n    wait\n}\nroutine p {\n    q()\n}\nroutine q {\n    goto x\n}\nroutine x {\n    y()\n}\nroutine y {\n    a += 1\n}\n",
            12,
            "x is a routine. Call it with x()",
        ),
    ] {
        for revision in [Revision::A, Revision::B] {
            let diagnostic = error(tally(revision, body));
            assert_eq!(
                (diagnostic.line, diagnostic.message.as_str()),
                (line, message),
                "{body}"
            );
        }
    }
}

#[test]
fn blocks_cannot_fall_into_a_routine() {
    let body = "\
entry main {
    a = 4
}
routine r {
    a = 9
}
block next {
    n = a
    wait
}
";
    for revision in [Revision::A, Revision::B] {
        let diagnostic = error(tally(revision, body));
        assert_eq!(
            (diagnostic.line, diagnostic.message.as_str()),
            (6, "main falls through into routine r. End main with a goto")
        );
    }
}

#[test]
fn loops_cannot_start_at_zero() {
    let for_loop = "\
entry main {
    a = 0
    for i = 0 downto 1 {
        a += 1
    }
    n = a
    wait
}
";
    let repeat = "\
entry main {
    a = 0
    i = 0
    repeat i {
        a += 1
    }
    n = a
    wait
}
";
    for (body, line, col) in [(for_loop, 5, 13), (repeat, 6, 5)] {
        for revision in [Revision::A, Revision::B] {
            let diagnostic = error(tally(revision, body));
            assert_eq!(
                (diagnostic.line, diagnostic.col, diagnostic.message.as_str()),
                (
                    line,
                    col,
                    "loops run at least once, so i must start at 1 or more"
                )
            );
        }
    }
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
    goto main
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
