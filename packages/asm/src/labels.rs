use std::collections::BTreeMap;

use ld60_machine::AsmErrorKind;

use crate::Operand;

pub struct Labels<'a> {
    map: &'a BTreeMap<String, u16>,
}

impl<'a> Labels<'a> {
    pub(crate) fn new(map: &'a BTreeMap<String, u16>) -> Self {
        Self { map }
    }

    pub fn get(&self, name: &str) -> Option<u16> {
        self.map.get(name).copied()
    }

    /// Resolves a number or a label name.
    pub fn value(&self, operand: &Operand) -> Result<i64, AsmErrorKind> {
        match operand {
            Operand::Number(n) => Ok(*n),
            Operand::Name(name) => self
                .get(name)
                .map(i64::from)
                .ok_or_else(|| AsmErrorKind::UnknownLabel { name: name.clone() }),
            Operand::Indirect { .. } => Err(AsmErrorKind::ExpectsValue),
        }
    }
}
