mod common;

use common::{build, code};
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
