use std::collections::BTreeMap;

use crate::ast::{Arm, Cond, Expr, ExprKind, File, Item, Stmt, StmtKind, Switch, SwitchKind};
use crate::diagnostic::Pos;
use crate::{Diagnostic, Flavor, Selection};

type Result<T> = std::result::Result<T, Diagnostic>;

struct Selector<'a> {
    selection: &'a Selection<'a>,
}

impl Selector<'_> {
    fn arm(&self, arm: Arm) -> Result<Arm> {
        Ok(Arm {
            pattern: self.expr(arm.pattern)?,
            ..arm
        })
    }

    fn cond(&self, cond: Cond) -> Result<Cond> {
        Ok(match cond {
            Cond::Compare { lhs, op, rhs } => Cond::Compare {
                lhs: self.expr(lhs)?,
                op,
                rhs: self.expr(rhs)?,
            },
            flag @ Cond::Flag { .. } => flag,
        })
    }

    fn expr(&self, expr: Expr) -> Result<Expr> {
        let pos = expr.pos;
        let boxed = |e: Box<Expr>| self.expr(*e).map(Box::new);
        let kind = match expr.kind {
            ExprKind::Binary(op, lhs, rhs) => ExprKind::Binary(op, boxed(lhs)?, boxed(rhs)?),
            ExprKind::Call(name, args) => ExprKind::Call(
                name,
                args.into_iter()
                    .map(|a| self.expr(a))
                    .collect::<Result<_>>()?,
            ),
            ExprKind::Index(name, index) => ExprKind::Index(name, boxed(index)?),
            ExprKind::Range(lo, hi) => ExprKind::Range(boxed(lo)?, boxed(hi)?),
            ExprKind::Switch(switch) => {
                let side = self.side(*switch);
                return match side.into_iter().next() {
                    Some(expr) => self.expr(expr),
                    None => Err(Diagnostic::error(pos, "this side of the switch is empty")),
                };
            }
            kind @ (ExprKind::Key(_) | ExprKind::Name(_) | ExprKind::Num(_)) => kind,
        };
        Ok(Expr { kind, pos })
    }

    fn items(&self, items: Vec<Item>) -> Result<Vec<Item>> {
        let mut out = Vec::new();
        for item in items {
            match item {
                Item::Switch(switch) => {
                    out.extend(self.items(self.side(switch))?);
                }
                Item::Block(mut block) => {
                    block.body = self.stmts(block.body)?;
                    out.push(Item::Block(block));
                }
                item => out.push(item),
            }
        }
        Ok(out)
    }

    fn side<T>(&self, switch: Switch<T>) -> Vec<T> {
        let selected = self.selection.switches.contains(&switch.name);
        let other = selected
            && match switch.kind {
                SwitchKind::Bug => self.selection.flavor == Flavor::Buggy,
                SwitchKind::Change => true,
                SwitchKind::Fix => self.selection.flavor == Flavor::Fixed,
            };
        if other { switch.other } else { switch.default }
    }

    fn stmt(&self, stmt: Stmt) -> Result<Vec<Stmt>> {
        let Stmt { kind, comment, pos } = stmt;
        let kind = match kind {
            StmtKind::Switch(switch) => return self.stmts(self.side(switch)),
            StmtKind::Assign { target, op, value } => StmtKind::Assign {
                target: self.expr(target)?,
                op,
                value: self.expr(value)?,
            },
            StmtKind::Call { name, args } => StmtKind::Call {
                name,
                args: args
                    .into_iter()
                    .map(|a| self.expr(a))
                    .collect::<Result<_>>()?,
            },
            StmtKind::For {
                counter,
                start,
                end,
                label,
                body,
            } => StmtKind::For {
                counter,
                start: self.expr(start)?,
                end: self.expr(end)?,
                label,
                body: self.stmts(body)?,
            },
            StmtKind::If { cond, then, els } => StmtKind::If {
                cond: self.cond(cond)?,
                then: self.stmts(then)?,
                els: self.stmts(els)?,
            },
            StmtKind::IfGoto { cond, target } => StmtKind::IfGoto {
                cond: self.cond(cond)?,
                target,
            },
            StmtKind::Match { scrutinee, arms } => StmtKind::Match {
                scrutinee: self.expr(scrutinee)?,
                arms: arms
                    .into_iter()
                    .map(|a| self.arm(a))
                    .collect::<Result<_>>()?,
            },
            StmtKind::Repeat {
                counter,
                label,
                body,
            } => StmtKind::Repeat {
                counter,
                label,
                body: self.stmts(body)?,
            },
            kind @ (StmtKind::Asm(_)
            | StmtKind::Comment(_)
            | StmtKind::Goto(_)
            | StmtKind::Label(_)
            | StmtKind::Return
            | StmtKind::Wait) => kind,
        };
        Ok(vec![Stmt { kind, comment, pos }])
    }

    fn stmts(&self, stmts: Vec<Stmt>) -> Result<Vec<Stmt>> {
        let mut out = Vec::new();
        for stmt in stmts {
            out.extend(self.stmt(stmt)?);
        }
        Ok(out)
    }
}

/// Every switch name in the file, on both sides of every switch.
#[derive(Default)]
struct Names {
    kinds: BTreeMap<String, SwitchKind>,
    conflicts: Vec<Diagnostic>,
}

impl Names {
    fn add<T>(&mut self, switch: &Switch<T>) {
        let kind = *self.kinds.entry(switch.name.clone()).or_insert(switch.kind);
        if kind != switch.kind {
            self.conflicts.push(Diagnostic::error(
                switch.pos,
                format!(
                    "switch '{}' is a {} switch elsewhere",
                    switch.name,
                    kind.keyword()
                ),
            ));
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Binary(_, lhs, rhs) | ExprKind::Range(lhs, rhs) => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Call(_, args) => args.iter().for_each(|a| self.expr(a)),
            ExprKind::Index(_, index) => self.expr(index),
            ExprKind::Switch(switch) => {
                self.add(switch);
                switch
                    .default
                    .iter()
                    .chain(&switch.other)
                    .for_each(|e| self.expr(e));
            }
            ExprKind::Key(_) | ExprKind::Name(_) | ExprKind::Num(_) => {}
        }
    }

    fn item(&mut self, item: &Item) {
        match item {
            Item::Switch(switch) => {
                self.add(switch);
                switch
                    .default
                    .iter()
                    .chain(&switch.other)
                    .for_each(|i| self.item(i));
            }
            Item::Block(block) => block.body.iter().for_each(|s| self.stmt(s)),
            Item::Cell(_) | Item::Comment(_) | Item::Const(_) | Item::Firmware(_) => {}
        }
    }

    fn cond(&mut self, cond: &Cond) {
        if let Cond::Compare { lhs, rhs, .. } = cond {
            self.expr(lhs);
            self.expr(rhs);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Switch(switch) => {
                self.add(switch);
                switch
                    .default
                    .iter()
                    .chain(&switch.other)
                    .for_each(|s| self.stmt(s));
            }
            StmtKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::Call { args, .. } => args.iter().for_each(|a| self.expr(a)),
            StmtKind::For {
                start, end, body, ..
            } => {
                self.expr(start);
                self.expr(end);
                body.iter().for_each(|s| self.stmt(s));
            }
            StmtKind::If { cond, then, els } => {
                self.cond(cond);
                then.iter().chain(els).for_each(|s| self.stmt(s));
            }
            StmtKind::IfGoto { cond, .. } => self.cond(cond),
            StmtKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                arms.iter().for_each(|a| self.expr(&a.pattern));
            }
            StmtKind::Repeat { body, .. } => body.iter().for_each(|s| self.stmt(s)),
            StmtKind::Asm(_)
            | StmtKind::Comment(_)
            | StmtKind::Goto(_)
            | StmtKind::Label(_)
            | StmtKind::Return
            | StmtKind::Wait => {}
        }
    }
}

/// Picks one side of every switch.
///
/// Fails on unknown switch names and on a name used with two kinds.
pub(crate) fn select(file: File, selection: &Selection<'_>) -> Result<File> {
    let mut names = Names::default();
    file.items.iter().for_each(|i| names.item(i));
    if let Some(conflict) = names.conflicts.into_iter().next() {
        return Err(conflict);
    }
    if let Some(name) = selection
        .switches
        .iter()
        .find(|n| !names.kinds.contains_key(*n))
    {
        return Err(Diagnostic::error(
            Pos::default(),
            format!("unknown switch '{name}'"),
        ));
    }
    let selector = Selector { selection };
    Ok(File {
        items: selector.items(file.items)?,
    })
}
