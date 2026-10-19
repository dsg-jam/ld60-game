use ld60_devices::{DeviceKind, LineTest};

const TALLY2_DOC: &str = include_str!("../../../docs/devices/tally2.md");
const SUMMA6_DOC: &str = include_str!("../../../docs/devices/summa6.md");
const LOCK4_DOC: &str = include_str!("../../../docs/devices/lock4.md");

/// The rows of the table under `## End of line tests`, as keys and display.
fn documented(doc: &str) -> Vec<(String, String)> {
    doc.lines()
        .skip_while(|line| *line != "## End of line tests")
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            let keys = cells[0].trim_matches('`');
            let keys = if keys == LineTest::POWER_ON { "" } else { keys };
            (keys.into(), cells[1].into())
        })
        .collect()
}

#[test]
fn docs_list_the_line_tests() {
    for (kind, doc) in [
        (DeviceKind::Tally2, TALLY2_DOC),
        (DeviceKind::Summa6, SUMMA6_DOC),
        (DeviceKind::Lock4, LOCK4_DOC),
    ] {
        let expected: Vec<_> = kind
            .line_tests()
            .iter()
            .map(|test| (test.keys.into(), test.display.into()))
            .collect();
        assert_eq!(documented(doc), expected, "{kind:?}");
    }
}
