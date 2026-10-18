use std::collections::{BTreeMap, BTreeSet};

use crate::backend::{Backend, Syntax};
use crate::emit::Line;

const MAX_LEN: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LabelId(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LabelName {
    /// Written in the source. Kept as is.
    Fixed(String),
    /// A preferred name. Changes on a clash.
    Hint(String),
    /// A join point: a prefix and a role taken from the next instruction.
    Join(String),
}

/// What a generated label is named after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Target(LabelId),
    Word(&'static str),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Labels(Vec<LabelName>);

impl Labels {
    pub(crate) fn add(&mut self, name: LabelName) -> LabelId {
        self.0.push(name);
        LabelId(self.0.len() - 1)
    }

    pub(crate) fn get(&self, id: LabelId) -> &LabelName {
        &self.0[id.0]
    }

    /// Names every label. Labels at the same place as a better one share its name.
    pub(crate) fn name(&self, lines: &[Line], backend: &dyn Backend) -> Names {
        let mut alias = BTreeMap::new();
        let mut next_instr = BTreeMap::new();
        let mut order = Vec::new();
        let mut group: Vec<LabelId> = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            match line {
                Line::Label(id) => {
                    group.push(*id);
                    order.push(*id);
                }
                Line::Instr(_) | Line::Raw(..) => {
                    for &id in &group {
                        next_instr.insert(id, index);
                    }
                    self.merge(&group, &mut alias);
                    group.clear();
                }
                Line::Blank | Line::Comment(_) => {}
            }
        }
        self.merge(&group, &mut alias);

        let mut taken: BTreeSet<String> = self
            .0
            .iter()
            .filter_map(|name| match name {
                LabelName::Fixed(name) => Some(name.clone()),
                _ => None,
            })
            .chain(lines.iter().filter_map(|line| match line {
                Line::Raw(raw, _) => {
                    asm_label(raw, backend.syntax()).map(|(name, _)| name.to_string())
                }
                _ => None,
            }))
            .collect();
        let mut names = BTreeMap::new();
        for (index, name) in self.0.iter().enumerate() {
            if let LabelName::Fixed(name) = name {
                names.insert(LabelId(index), name.clone());
            }
        }
        for id in order {
            if names.contains_key(&id) || alias.contains_key(&id) {
                continue;
            }
            let wanted = match self.get(id) {
                LabelName::Fixed(name) | LabelName::Hint(name) => name.clone(),
                LabelName::Join(prefix) => {
                    let next = next_instr.get(&id).and_then(|&i| match &lines[i] {
                        Line::Instr(instr) => Some(backend.label_role(instr)),
                        _ => None,
                    });
                    let word = match next {
                        Some(Role::Target(target)) => {
                            names.get(&target).cloned().unwrap_or_else(|| "skip".into())
                        }
                        Some(Role::Word(word)) => word.into(),
                        None => "skip".into(),
                    };
                    format!("{prefix}{word}")
                }
            };
            let name = unique(&wanted, &taken);
            taken.insert(name.clone());
            names.insert(id, name);
        }
        Names { names, alias }
    }

    /// Points every weaker label of a group at the strongest one.
    fn merge(&self, group: &[LabelId], alias: &mut BTreeMap<LabelId, LabelId>) {
        let rank = |id: &LabelId| match self.get(*id) {
            LabelName::Fixed(_) => 0,
            LabelName::Hint(_) => 1,
            LabelName::Join(_) => 2,
        };
        let Some(&best) = group.iter().min_by_key(|id| rank(id)) else {
            return;
        };
        for &id in group {
            if id != best && rank(&id) > 0 {
                alias.insert(id, best);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Names {
    names: BTreeMap<LabelId, String>,
    alias: BTreeMap<LabelId, LabelId>,
}

impl Names {
    /// The printed name of `id`.
    pub(crate) fn get(&self, id: LabelId) -> &str {
        let id = self.alias.get(&id).copied().unwrap_or(id);
        self.names.get(&id).map_or("?", String::as_str)
    }

    /// Whether `id` is printed as its own label.
    pub(crate) fn is_shown(&self, id: LabelId) -> bool {
        !self.alias.contains_key(&id)
    }
}

fn unique(wanted: &str, taken: &BTreeSet<String>) -> String {
    let base: String = wanted.chars().take(MAX_LEN).collect();
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n: u32| {
            let digits = n.to_string();
            let keep = base.len().min(MAX_LEN - digits.len()).max(1);
            format!("{}{digits}", &base[..keep])
        })
        .find(|name| !taken.contains(name))
        .unwrap_or(base)
}

/// The label and the rest of an `asm { }` line, if it starts with one.
pub(crate) fn asm_label(line: &str, syntax: Syntax) -> Option<(&str, &str)> {
    let code = line.split(syntax.comment).next().unwrap_or_default();
    if !code.contains(syntax.label_end) {
        return None;
    }
    let (name, rest) = line.split_once(syntax.label_end)?;
    let name = name.trim();
    let valid = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then_some((name, rest.trim()))
}
