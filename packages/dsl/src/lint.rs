use std::collections::BTreeMap;

use crate::ast::BlockKind;
use crate::backend::Features;
use crate::diagnostic::{Pos, Severity};
use crate::hir::{Block, Cond, Node, Prim, Program, Stmt};
use crate::labels::LabelId;
use crate::{Diagnostic, Flavor};

/// What a routine does to the machine state, callees included.
#[derive(Clone, Copy, Debug, Default)]
struct Effects {
    writes_index: bool,
    /// Stack entries in use while the routine runs, its own return address included.
    depth: u8,
}

struct Linter<'a> {
    program: &'a Program,
    severity: Severity,
    effects: BTreeMap<LabelId, Effects>,
    diagnostics: Vec<Diagnostic>,
}

impl Linter<'_> {
    fn a_clobber(&mut self, nodes: &[Node]) {
        for pair in nodes.windows(2) {
            if matches!(pair[0].stmt, Stmt::Prim(Prim::Move { .. })) && reads_a(&pair[1]) {
                self.diagnostics.push(Diagnostic::warning(
                    pair[1].pos,
                    "the copy above changed a, but this line reads a",
                ));
            }
        }
        for node in nodes {
            match &node.stmt {
                Stmt::If { then, els, .. } => {
                    self.a_clobber(then);
                    self.a_clobber(els);
                }
                Stmt::Inlined { body, .. } | Stmt::Repeat { body, .. } => self.a_clobber(body),
                _ => {}
            }
        }
    }

    /// Effects of calling `routine`. Fails on recursion.
    fn effects(&mut self, routine: LabelId, stack: &mut Vec<LabelId>) -> Result<Effects, ()> {
        if let Some(&effects) = self.effects.get(&routine) {
            return Ok(effects);
        }
        let Some(block) = self.program.routine(routine) else {
            return Ok(Effects::default());
        };
        if stack.contains(&routine) {
            return Err(());
        }
        stack.push(routine);
        let mut effects = Effects {
            writes_index: false,
            depth: 1,
        };
        let mut calls = Vec::new();
        Node::visit(&block.body, &mut |node| match &node.stmt {
            Stmt::Prim(Prim::SetIndex(_)) | Stmt::Repeat { .. } => effects.writes_index = true,
            Stmt::Prim(Prim::Call(target)) => calls.push(*target),
            _ => {}
        });
        for target in calls {
            let callee = self.effects(target, stack)?;
            effects.writes_index |= callee.writes_index;
            effects.depth = effects.depth.max(callee.depth.saturating_add(1));
        }
        stack.pop();
        self.effects.insert(routine, effects);
        Ok(effects)
    }

    fn report(&mut self, pos: Pos, message: String) {
        self.diagnostics
            .push(Diagnostic::new(pos, self.severity, message));
    }

    fn walk(&mut self, block: &Block, nodes: &[Node], in_loop: bool, features: &Features) {
        for node in nodes {
            match &node.stmt {
                Stmt::Prim(Prim::Call(target)) => {
                    let name = self
                        .program
                        .routine(*target)
                        .map_or("?", |r| r.name.as_str());
                    let Ok(effects) = self.effects(*target, &mut Vec::new()) else {
                        self.diagnostics
                            .push(Diagnostic::error(node.pos, format!("{name} calls itself")));
                        continue;
                    };
                    if in_loop && effects.writes_index {
                        self.report(node.pos, format!("{name} changes i inside a loop"));
                    }
                    if let Some(limit) = features.stack_depth
                        && block.kind != BlockKind::Routine
                        && effects.depth > limit
                    {
                        self.report(
                            node.pos,
                            format!(
                                "this call nests {} deep, but the stack holds {limit}",
                                effects.depth
                            ),
                        );
                    }
                }
                Stmt::Repeat { body, .. } => {
                    if in_loop {
                        self.report(node.pos, "loops cannot nest, there is only one i".into());
                    }
                    self.walk(block, body, true, features);
                }
                Stmt::If { then, els, .. } => {
                    self.walk(block, then, in_loop, features);
                    self.walk(block, els, in_loop, features);
                }
                Stmt::Inlined { body, .. } => self.walk(block, body, in_loop, features),
                _ => {}
            }
        }
    }
}

/// Checks register and stack use. Returns the warnings, or the errors if there are any.
///
/// Rule breaks are errors in the fixed firmware and warnings in the buggy one,
/// where breaking a rule is often the bug.
pub(crate) fn lint(
    program: &Program,
    features: &Features,
    flavor: Flavor,
) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let mut linter = Linter {
        program,
        severity: match flavor {
            Flavor::Buggy => Severity::Warning,
            Flavor::Fixed => Severity::Error,
        },
        effects: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    for block in &program.blocks {
        linter.walk(block, &block.body, false, features);
        linter.a_clobber(&block.body);
    }
    let diagnostics = linter.diagnostics;
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        Err(diagnostics
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect())
    } else {
        Ok(diagnostics)
    }
}

fn reads_a(node: &Node) -> bool {
    match &node.stmt {
        Stmt::Prim(prim) => matches!(
            prim,
            Prim::AddCarry(_)
                | Prim::Branch(Cond::Compare(..), _)
                | Prim::Dec
                | Prim::Inc
                | Prim::Store(_)
                | Prim::SubCarry(_)
        ),
        Stmt::If { cond, .. } => matches!(cond, Cond::Compare(..)),
        _ => false,
    }
}
