use crate::DeviceKind;

const TALLY2_TESTS: [LineTest; 4] = [
    LineTest::new("", "00"),
    LineTest::new("C +", "01"),
    LineTest::new("C +++", "03"),
    LineTest::new("C +++C", "00"),
];
const SUMMA6_TESTS: [LineTest; 7] = [
    LineTest::new("", "000000"),
    LineTest::new("C 123456", "123456"),
    LineTest::new("C 7890", "007890"),
    LineTest::new("C 12+34=", "000046"),
    LineTest::new("C 99+1=", "000100"),
    LineTest::new("C 1000-1=", "000999"),
    LineTest::new("C 50-8=", "000042"),
];
const LOCK4_TESTS: [LineTest; 8] = [
    LineTest::new("", "0000 SHUT"),
    LineTest::new("C 1234", "1234 SHUT"),
    LineTest::new("C #", "0000 OPEN"),
    LineTest::new("C 12#", "0012 SHUT"),
    LineTest::new("C #C", "0000 SHUT"),
    LineTest::new("C 1#2#3#", "E"),
    LineTest::new("C #*", "0000 OPEN SHUT"),
    LineTest::new("C #*42# 42#", "0042 OPEN"),
];
const SCORE5_TESTS: [LineTest; 9] = [
    LineTest::new("", "00100"),
    LineTest::new("C H", "01100"),
    LineTest::new("C GG", "00102"),
    LineTest::new("C 3H 2G", "03102"),
    LineTest::new("C 2HH", "03100"),
    LineTest::new("C 5G 5G", "00110"),
    LineTest::new("C 9H 9H 9H 9H 9H 9H 9H 9H 9H 9H 9H 9H", "99100"),
    LineTest::new("C PP", "00300"),
    LineTest::new("C PPPPP", "00400"),
];

/// A check from the factory checklist of a device model.
///
/// It names keys, not key codes, so it holds for every board of the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineTest {
    /// Keys pressed after power on. Whitespace only groups them.
    pub keys: &'static str,
    /// The expected [`Readout`](crate::Readout) as text.
    pub display: &'static str,
}

impl LineTest {
    /// Printed in place of the keys of a test without any.
    pub const POWER_ON: &'static str = "(power on)";

    const fn new(keys: &'static str, display: &'static str) -> Self {
        Self { keys, display }
    }

    /// The keys as printed on the checklist.
    pub fn steps(&self) -> &'static str {
        if self.keys.is_empty() {
            Self::POWER_ON
        } else {
            self.keys
        }
    }
}

impl DeviceKind {
    /// The end of line tests every unit of the model passes at the factory.
    pub fn line_tests(self) -> &'static [LineTest] {
        match self {
            Self::Tally2 => &TALLY2_TESTS,
            Self::Summa6 => &SUMMA6_TESTS,
            Self::Lock4 => &LOCK4_TESTS,
            Self::Score5 => &SCORE5_TESTS,
        }
    }
}
