mod common;

use common::{SUMMA, build};
use ld60_calco::Revision;
use ld60_devices::DeviceKind;
use ld60_dsl::{Diagnostic, Flavor, Output, Severity};

fn summa(body: &str, flavor: Flavor) -> Result<Output, Vec<Diagnostic>> {
    let source =
        format!("firmware \"T\" for summa6\ncells d[6] at M1\ncell x at M7\ncell y at M8\n{body}");
    build(&source, Revision::B, DeviceKind::Summa6, flavor, &[])
}

fn messages(diagnostics: &[Diagnostic]) -> Vec<(usize, Severity, &str)> {
    diagnostics
        .iter()
        .map(|d| (d.line, d.severity, d.message.as_str()))
        .collect()
}

#[test]
fn deep_calls_are_errors_when_fixed_and_warnings_when_buggy() {
    let body = "\
entry main {
    one()
    wait
    goto main
}
routine one {
    two()
}
routine two {
    three()
}
routine three {
    nop()
}
";
    let errors = summa(body, Flavor::Fixed).unwrap_err();
    assert_eq!(
        messages(&errors),
        [(
            6,
            Severity::Error,
            "this call nests 3 deep, but the stack holds 2"
        )]
    );
    let output = summa(body, Flavor::Buggy).unwrap();
    assert_eq!(
        messages(&output.warnings),
        [(
            6,
            Severity::Warning,
            "this call nests 3 deep, but the stack holds 2"
        )]
    );
}

#[test]
fn the_level_7_bug_trips_the_stack_check() {
    let output = build(
        SUMMA,
        Revision::B,
        DeviceKind::Summa6,
        Flavor::Buggy,
        &["deep-call"],
    )
    .unwrap();
    assert_eq!(output.warnings.len(), 3, "{:?}", output.warnings);
    let errors = build(
        SUMMA,
        Revision::B,
        DeviceKind::Summa6,
        Flavor::Fixed,
        &["deep-call"],
    );
    assert!(errors.is_ok());
}

#[test]
fn loops_do_not_nest() {
    let body = "\
entry main {
    for i = 6 downto 1 {
        for i = 2 downto 1 {
            d[i] = a
        }
    }
    wait
}
";
    let errors = summa(body, Flavor::Fixed).unwrap_err();
    assert_eq!(
        messages(&errors),
        [(7, Severity::Error, "loops cannot nest, there is only one i")]
    );
}

#[test]
fn calls_in_loops_must_keep_i() {
    let body = "\
entry main {
    for i = 6 downto 1 {
        clear()
    }
    wait
    goto main
}
routine clear {
    for i = 6 downto 1 {
        d[i] = a
    }
}
";
    let errors = summa(body, Flavor::Fixed).unwrap_err();
    assert_eq!(
        messages(&errors),
        [(7, Severity::Error, "clear changes i inside a loop")]
    );
    assert_eq!(summa(body, Flavor::Buggy).unwrap().warnings.len(), 1);
}

#[test]
fn copies_that_lose_a_warn() {
    let body = "\
entry main {
    a = 1
    x = y
    d[1] = a
    wait
}
";
    for flavor in [Flavor::Buggy, Flavor::Fixed] {
        let output = summa(body, flavor).unwrap();
        assert_eq!(
            messages(&output.warnings),
            [(
                8,
                Severity::Warning,
                "the copy above changed a, but this line reads a"
            )]
        );
    }
}

#[test]
fn recursion_is_an_error() {
    let body = "\
entry main {
    again()
    goto main
}
routine again {
    again()
}
";
    for flavor in [Flavor::Buggy, Flavor::Fixed] {
        let errors = summa(body, flavor).unwrap_err();
        assert_eq!(errors[0].message, "again calls itself");
    }
}
