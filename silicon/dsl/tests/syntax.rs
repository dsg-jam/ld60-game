mod common;

use common::{error, tally};
use ld60_dsl::Severity;

const HEAD: &str = "firmware \"Test\" for tally2\n";

fn source(body: &str) -> String {
    format!("{HEAD}{body}")
}

#[test]
fn lexer_errors_have_positions() {
    for (body, line, col, message) in [
        (
            "entry main {\n    a = 300\n}\n",
            3,
            9,
            "invalid number '300'",
        ),
        (
            "entry main {\n    a = $\n}\n",
            3,
            9,
            "unexpected character '$'",
        ),
        (
            "entry main {\n    a = 'C\n}\n",
            3,
            9,
            "expected a key like '+'",
        ),
        ("entry main {\n    asm {\n", 3, 5, "asm block is not closed"),
    ] {
        let diagnostic = error(tally(&source(body)));
        assert_eq!(
            (diagnostic.line, diagnostic.col, diagnostic.message.as_str()),
            (line, col, message),
            "{body}"
        );
        assert_eq!(diagnostic.severity, Severity::Error);
    }
}

#[test]
fn parser_errors_have_positions() {
    for (body, line, col, message) in [
        (
            "entry main {\n    wait wait\n}\n",
            3,
            10,
            "expected the end of the line, found 'wait'",
        ),
        (
            "entry main {\n    a == 1\n}\n",
            3,
            7,
            "expected '=', '+=' or '-=', found '=='",
        ),
        (
            "entry main {\n    wait\n",
            4,
            1,
            "expected '}', found the end of the file",
        ),
        (
            "loop main {}\n",
            2,
            1,
            "expected a declaration, found 'loop'",
        ),
        (
            "bug \"x\" { broken { } }\n",
            2,
            11,
            "expected 'fixed' or 'buggy', found 'broken'",
        ),
        (
            "entry main {\n    if a < goto main\n}\n",
            3,
            12,
            "expected a value, found 'goto'",
        ),
    ] {
        let diagnostic = error(tally(&source(body)));
        assert_eq!(
            (diagnostic.line, diagnostic.col, diagnostic.message.as_str()),
            (line, col, message),
            "{body}"
        );
    }
}

#[test]
fn name_errors_have_positions() {
    for (body, line, message) in [
        (
            "entry main {\n    goto nowhere\n}\n",
            3,
            "unknown label 'nowhere'",
        ),
        (
            "entry main {\n    a = count\n}\n",
            3,
            "unknown cell 'count'",
        ),
        ("entry main {\n    a = '7'\n}\n", 3, "tally2 has no key '7'"),
        ("entry main {\n    fly()\n}\n", 3, "unknown routine 'fly'"),
        (
            "entry main {\n    return\n}\n",
            3,
            "return is only allowed in a routine",
        ),
        (
            "cell x at M3\ncell y at M3\nentry main {\n    wait\n}\n",
            3,
            "'y' overlaps the cell at M3",
        ),
        (
            "cells x[4] at MD\nentry main {\n    wait\n}\n",
            2,
            "'MD' does not fit in RAM",
        ),
        (
            "entry main {\n    wait\n}\nblock main {\n    wait\n}\n",
            5,
            "label 'main' is defined twice",
        ),
        (
            "block main {\n    wait\n}\nentry reset {\n    wait\n}\n",
            5,
            "the entry block must come first",
        ),
    ] {
        let diagnostic = error(tally(&source(body)));
        assert_eq!(
            (diagnostic.line, diagnostic.message.as_str()),
            (line, message),
            "{body}"
        );
    }
}

#[test]
fn firmware_must_match_the_device() {
    let diagnostic = error(tally(
        "firmware \"T\" for summa6\nentry main {\n    wait\n}\n",
    ));
    assert_eq!(
        diagnostic.message,
        "firmware is for summa6, but the target is tally2"
    );
}

#[test]
fn diagnostics_print_their_position() {
    let diagnostic = error(tally(&source("entry main {\n    goto x\n}\n")));
    assert_eq!(diagnostic.to_string(), "3:5: error: unknown label 'x'");
}
