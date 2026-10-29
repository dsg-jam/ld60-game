mod common;

use std::collections::VecDeque;

use common::{SUMMA, build, code, error, target};
use ld60_calco::{ChipInfo, Cpu, DateCode, Revision};
use ld60_devices::DeviceKind;
use ld60_dsl::{Diagnostic, Flavor, Output, compile_pair};
use ld60_machine::{Bus, Cpu as _, Isa, Step};

struct Board {
    rom: Vec<u16>,
    keys: VecDeque<u8>,
}

impl Bus for Board {
    fn port_in(&mut self, _port: u8) -> u8 {
        self.keys.pop_front().unwrap_or(0xFF)
    }

    fn port_out(&mut self, _port: u8, _value: u8) {}

    fn rom(&self, addr: u16) -> u16 {
        self.rom.get(usize::from(addr)).copied().unwrap_or(0)
    }

    fn take_wake(&mut self) -> bool {
        !self.keys.is_empty()
    }
}

fn tally(revision: Revision, body: &str) -> Result<Output, Vec<Diagnostic>> {
    let source = format!("firmware \"T\" for tally2\ncell n at M1\n{body}");
    build(&source, revision, DeviceKind::Tally2, Flavor::Fixed, &[])
}

/// RAM after running `body` until it halts with no keys left.
fn run(revision: Revision, body: &str, keys: &[u8]) -> Vec<u8> {
    let output = tally(revision, body).unwrap();
    let chip = ChipInfo {
        revision,
        date_code: DateCode::new(76, 30),
    };
    let image = revision.isa().assemble(&output.asm).unwrap().image;
    let mut board = Board {
        rom: image.words().to_vec(),
        keys: keys.iter().copied().collect(),
    };
    let mut cpu = Cpu::new(chip);
    for _ in 0..10_000 {
        if cpu.step(&mut board) == Step::Halted && board.keys.is_empty() {
            return cpu.ram().cells().to_vec();
        }
    }
    panic!("no halt\n{}", output.asm);
}

/// Runs `body` on Rev A and Rev B and checks that both end with the same RAM.
fn same_on_a_and_b(body: &str, keys: &[u8]) -> Vec<u8> {
    let a = run(Revision::A, body, keys);
    assert_eq!(a, run(Revision::B, body, keys), "{body}");
    a
}

#[test]
fn rev_a_and_rev_b_builds_agree() {
    let branches = "\
cell k at M2
entry main {
    a = KEY
    k = a
    if a == 5 goto go
go:
    a = k
    if !zero goto yes
    a = 2
    goto done
yes:
    a = 1
done:
    n = a
    wait
    goto main
}
";
    assert_eq!(same_on_a_and_b(branches, &[0x05])[..3], [0, 2, 5]);
    let calls = "\
cell m at M2
entry main {
    a = KEY
    s()
    n = a
    a = KEY
    s()
    m = a
    wait
    goto main
}
routine s {
    t()
    if a == 3 goto x
    a += 1
    return
x:
    a = 9
}
routine t {
    if a == 0 {
        return
    }
    a += 1
}
";
    let ram = same_on_a_and_b(calls, &[0x02, 0x00]);
    assert_eq!(ram[1..3], [9, 1]);
    let unrolled = "\
cells arr[3] at M4
entry main {
    a = 0
    arr[2] = a
    a = 7
    arr[1] = a
    arr[3] = a
    for i = 3 downto 1 as lp {
        a = arr[i]
        if a == 0 {
            a = 5
        }
        arr[i] = a
    }
    wait
    goto main
}
";
    assert_eq!(same_on_a_and_b(unrolled, &[])[4..7], [7, 5, 7]);
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
        "main:\ns:      INC\n        INC\n        INC\n        ST   M1\n        WAIT\n        JMP  main\n"
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
        WAIT
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
fn oversized_firmware_gives_one_error() {
    let calls = "    s()\n".repeat(60);
    let body = format!(
        "entry main {{\n{calls}    wait\n    goto main\n}}\nroutine s {{\n{}}}\n",
        "    a = n\n".repeat(5)
    );
    let errors = tally(Revision::A, &body).unwrap_err();
    let found: Vec<_> = errors
        .iter()
        .map(|d| (d.line, d.message.as_str()))
        .collect();
    assert_eq!(found, [(69, "firmware is 303 bytes, the ROM holds 256")]);
}

#[test]
fn assembler_errors_point_at_the_source() {
    let body = "entry main {\n    wait\n    asm {\n        FOO\n    }\n    goto main\n}\n";
    let errors = tally(Revision::B, body).unwrap_err();
    let found: Vec<_> = errors
        .iter()
        .map(|d| (d.line, d.message.as_str()))
        .collect();
    assert_eq!(found, [(5, "generated code: unknown mnemonic 'FOO'")]);
}

#[test]
fn a_switch_name_has_one_kind() {
    let source = "\
firmware \"T\" for tally2
fix \"x\" {
    after {
        cell foo at M4
    }
}
entry main {
    bug \"x\" {
        buggy {
            a = 1
        }
    }
    change \"x\" {
        after {
            a = 2
            fix \"y\" {
                after {
                    a = 3
                }
            }
        }
    }
    a = foo
    wait
}
";
    let switches = ["x".to_string()];
    let errors =
        compile_pair(source, &target(Revision::B, DeviceKind::Tally2), &switches).unwrap_err();
    let found: Vec<_> = errors
        .iter()
        .map(|d| (d.line, d.message.as_str()))
        .collect();
    assert_eq!(
        found,
        [(8, "buggy firmware: switch 'x' is a fix switch elsewhere")]
    );
}

#[test]
fn recursion_fails_on_every_revision() {
    let uncalled = "entry main {\n    wait\n    goto main\n}\nroutine z {\n    z()\n}\n";
    let cycle = "\
entry main {
    p()
    goto main
}
routine p {
    q()
}
routine q {
    p()
}
routine z {
    z()
}
";
    for revision in [Revision::A, Revision::B] {
        let errors = tally(revision, uncalled).unwrap_err();
        let found: Vec<_> = errors
            .iter()
            .map(|d| (d.line, d.message.as_str()))
            .collect();
        assert_eq!(found, [(8, "z calls itself")]);
        let errors = tally(revision, cycle).unwrap_err();
        let found: Vec<_> = errors
            .iter()
            .map(|d| (d.line, d.message.as_str()))
            .collect();
        assert_eq!(found, [(11, "p calls itself"), (14, "z calls itself")]);
    }
}

#[test]
fn empty_arrays_are_named() {
    let body = "cells x[0] at M4\nentry main {\n    wait\n    goto main\n}\n";
    let diagnostic = error(tally(Revision::B, body));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (3, "'x' needs at least 1 element")
    );
}

#[test]
fn loops_longer_than_their_array_warn() {
    for body in [
        "cells x[2] at M4\nentry main {\n    for i = 3 downto 1 {\n        x[i] = a\n    }\n    wait\n    goto main\n}\n",
        "cells x[2] at M4\nentry main {\n    i = 3\n    carry = 0\n    repeat i {\n        x[i] = a\n    }\n    wait\n    goto main\n}\n",
    ] {
        let output = tally(Revision::B, body).unwrap();
        let found: Vec<_> = output.warnings.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(
            found,
            ["the loop runs 3 times, but x has 2 elements"],
            "{body}"
        );
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

#[test]
fn returns_belong_to_routines_and_stacks_are_counted() {
    let goto_block = "\
entry main {
    p()
    wait
    goto main
}
routine p {
    q()
}
routine q {
    goto r
}
block r {
    w()
    return
}
routine w {
    a += 1
}
";
    let diagnostic = error(tally(Revision::B, goto_block));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (16, "return is only allowed in a routine")
    );
    let deep = "\
entry main {
    p()
    n = a
    wait
    goto main
}
routine p {
    q()
}
routine q {
    y()
}
routine y {
    a += 1
}
";
    let diagnostic = error(tally(Revision::B, deep));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (4, "this call nests 3 deep, but the stack holds 2")
    );
    assert!(tally(Revision::A, deep).is_ok());
}

#[test]
fn a_routine_loop_needs_a_known_count_on_rev_a() {
    let body = "\
cells arr[3] at M4
entry main {
    i = 3
    s()
    wait
    goto main
}
routine s {
    repeat i {
        a = arr[i]
        a += 1
        arr[i] = a
    }
}
";
    let diagnostic = error(tally(Revision::A, body));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (
            11,
            "CalCo Rev A cannot run this loop. Set i to a number right before it so it can be \
             unrolled"
        )
    );
    assert!(tally(Revision::B, body).is_ok());
}

#[test]
fn labels_cannot_take_port_names() {
    let body = "entry main {\n    goto REV\nREV:\n    wait\n}\n";
    let errors = tally(Revision::B, body).unwrap_err();
    let found: Vec<_> = errors
        .iter()
        .map(|d| (d.line, d.message.as_str()))
        .collect();
    assert_eq!(found, [(5, "'REV' is already used")]);
}

#[test]
fn unrolled_comments_appear_once() {
    let body = "\
entry main {
    for i = 3 downto 1 { ; count
        a += 1
    }
    n = a ; save
    wait
    goto main
}
";
    let output = tally(Revision::A, body).unwrap();
    assert_eq!(
        code(&output),
        "\
main:
        ; count
        INC
        INC
        INC
        ST   M1             ; save
        WAIT
        JMP  main
"
    );
}

#[test]
fn long_straight_code_fits() {
    let body = format!("entry main {{\n{}    wait\n}}\n", "    a = n\n".repeat(135));
    for revision in [Revision::A, Revision::B] {
        assert!(tally(revision, &body).is_ok());
    }
}

#[test]
fn tabs_and_crlf_give_the_same_output() {
    let build_summa = |source: &str| {
        build(source, Revision::B, DeviceKind::Summa6, Flavor::Fixed, &[])
            .unwrap()
            .asm
    };
    let expected = build_summa(SUMMA);
    assert_eq!(build_summa(&SUMMA.replace('\n', "\r\n")), expected);
    assert_eq!(build_summa(&SUMMA.replace("    ", "\t")), expected);
}
