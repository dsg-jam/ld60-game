mod common;

use common::{SUMMA, build, error, target};
use ld60_calco::Revision;
use ld60_devices::DeviceKind;
use ld60_dsl::{Flavor, compile_pair};

const SOURCE: &str = "\
firmware \"T\" for tally2
cell n at M1
bug \"count\" {
    buggy {
        cell spare at M2
    }
}
entry main {
    a = bug \"start\" { fixed { 1 } buggy { 2 } }
    fix \"store\" {
        before {
            n = a
        }
        after {
            spare = a
        }
    }
    bug \"count\" {
        buggy {
            spare = a
        }
    }
    change \"halt\" {
        after {
            wait
        }
    }
    goto main
}
";

fn lines(flavor: Flavor, switches: &[&str]) -> Vec<String> {
    let output = build(SOURCE, Revision::B, DeviceKind::Tally2, flavor, switches);
    match output {
        Ok(output) => output
            .asm
            .lines()
            .filter(|l| l.starts_with("main:") || l.starts_with("        "))
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect(),
        Err(errors) => panic!("{errors:?}"),
    }
}

#[test]
fn no_switch_takes_the_default_sides() {
    for flavor in [Flavor::Buggy, Flavor::Fixed] {
        assert_eq!(lines(flavor, &[]), ["main: LDI A, 1", "ST M1", "JMP main"]);
    }
}

#[test]
fn bug_switches_change_the_buggy_firmware() {
    assert_eq!(
        lines(Flavor::Buggy, &["start"]),
        ["main: LDI A, 2", "ST M1", "JMP main"]
    );
    assert_eq!(
        lines(Flavor::Fixed, &["start"]),
        ["main: LDI A, 1", "ST M1", "JMP main"]
    );
}

#[test]
fn bug_switches_flip_together() {
    assert_eq!(
        lines(Flavor::Buggy, &["count"]),
        ["main: LDI A, 1", "ST M1", "ST M2", "JMP main"]
    );
}

#[test]
fn fix_switches_change_the_fixed_firmware() {
    assert_eq!(
        lines(Flavor::Buggy, &["count", "store"]),
        ["main: LDI A, 1", "ST M1", "ST M2", "JMP main"]
    );
    let fixed = build(
        SOURCE,
        Revision::B,
        DeviceKind::Tally2,
        Flavor::Fixed,
        &["store"],
    );
    let error = error(fixed);
    assert_eq!(error.message, "unknown cell 'spare'");
}

#[test]
fn change_switches_change_both() {
    for flavor in [Flavor::Buggy, Flavor::Fixed] {
        assert_eq!(
            lines(flavor, &["halt"]),
            ["main: LDI A, 1", "ST M1", "WAIT", "JMP main"]
        );
    }
}

#[test]
fn unknown_switches_are_errors() {
    let switches = ["typo".to_string()];
    let errors =
        compile_pair(SOURCE, &target(Revision::B, DeviceKind::Tally2), &switches).unwrap_err();
    assert_eq!(errors[0].message, "buggy firmware: unknown switch 'typo'");
}

#[test]
fn nested_switch_names_are_known() {
    let switches = ["nine-key".to_string(), "deep-call".to_string()];
    let pair = compile_pair(SUMMA, &target(Revision::B, DeviceKind::Summa6), &switches).unwrap();
    assert!(pair.buggy.asm.contains("CPI  9\n"));
    assert!(pair.buggy.asm.contains("CALL adig"));
    assert!(!pair.fixed.asm.contains("adig"));
}

#[test]
fn an_empty_expression_side_is_an_error() {
    let source = "firmware \"T\" for tally2\nentry main {\n    a = bug \"x\" { buggy { 1 } }\n}\n";
    let diagnostic = error(build(
        source,
        Revision::B,
        DeviceKind::Tally2,
        Flavor::Fixed,
        &[],
    ));
    assert_eq!(
        (diagnostic.line, diagnostic.message.as_str()),
        (3, "this side of the switch is empty")
    );
}
