use crate::Diagnostic;
use crate::ast::BlockKind;
use crate::backend::Backend;
use crate::diagnostic::Pos;
use crate::hir::{Block, Node, Prim, Program, Stmt};
use crate::labels::{LabelId, LabelName, Labels};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operand {
    Label(LabelId),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Instr {
    pub(crate) mnemonic: &'static str,
    pub(crate) operands: Vec<Operand>,
    pub(crate) comment: Option<String>,
    /// Bytes or words the instruction takes in ROM.
    pub(crate) size: usize,
    pub(crate) pos: Pos,
    /// Control never falls through to the next line.
    pub(crate) stops: bool,
    /// A generated jump that can go if its target is the next line.
    pub(crate) droppable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Line {
    Blank,
    Comment(String),
    Instr(Instr),
    Label(LabelId),
    Raw(String, Pos),
}

/// Collects the output lines. Backends add instructions and labels here.
pub(crate) struct Emitter {
    pub(crate) lines: Vec<Line>,
    pub(crate) labels: Labels,
    comment: Option<String>,
    droppable: bool,
    prefix: String,
    pos: Pos,
}

impl Emitter {
    /// A new join label named after the block and the instruction it lands on.
    pub(crate) fn join(&mut self) -> LabelId {
        self.labels.add(LabelName::Join(self.prefix.clone()))
    }

    pub(crate) fn op(&mut self, mnemonic: &'static str, operands: Vec<Operand>, size: usize) {
        self.lines.push(Line::Instr(Instr {
            mnemonic,
            operands,
            comment: self.comment.take(),
            size,
            pos: self.pos,
            stops: false,
            droppable: self.droppable,
        }));
    }

    pub(crate) fn place(&mut self, label: LabelId) {
        self.lines.push(Line::Label(label));
    }

    fn mark_stop(&mut self, from: usize) {
        if let Some(Line::Instr(instr)) = self.lines[from..]
            .iter_mut()
            .rev()
            .find(|l| matches!(l, Line::Instr(_)))
        {
            instr.stops = true;
        }
    }
}

struct Walker<'a> {
    backend: &'a dyn Backend,
    out: Emitter,
}

impl Walker<'_> {
    /// Emits a block. A blank line goes before it unless it follows the
    /// entry block or is reached by falling through.
    fn block(&mut self, block: &Block, previous: Option<BlockKind>) -> Result<(), Diagnostic> {
        let stopped = self
            .out
            .lines
            .iter()
            .rev()
            .find_map(|l| match l {
                Line::Instr(instr) => Some(instr.stops),
                _ => None,
            })
            .unwrap_or(false);
        if stopped && previous.is_some_and(|kind| kind != BlockKind::Entry) {
            self.out.lines.push(Line::Blank);
        }
        self.out.prefix = block.name.chars().take(1).collect();
        self.out.place(block.label);
        self.nodes(&block.body)?;
        if block.kind == BlockKind::Routine && !Node::ends_in_transfer(&block.body) {
            self.prim(&Prim::Return, block.pos)?;
        }
        Ok(())
    }

    fn node(&mut self, node: &Node) -> Result<(), Diagnostic> {
        if node.comment.is_some() {
            self.out.comment.clone_from(&node.comment);
        }
        let pos = node.pos;
        self.out.pos = pos;
        match &node.stmt {
            Stmt::Asm(lines) => {
                for line in lines {
                    self.out.lines.push(Line::Raw(line.clone(), pos));
                }
            }
            Stmt::Comment(text) => self.out.lines.push(Line::Comment(text.clone())),
            Stmt::If { cond, then, els } => {
                if els.is_empty() {
                    let join = self.out.join();
                    self.prim(&Prim::Branch(cond.inverse(), join), pos)?;
                    self.nodes(then)?;
                    self.out.place(join);
                } else {
                    let taken = self.out.join();
                    self.prim(&Prim::Branch(cond.clone(), taken), pos)?;
                    self.nodes(els)?;
                    let join = self.out.join();
                    if !Node::ends_in_transfer(els) {
                        self.out.droppable = true;
                        self.prim(&Prim::Jump(join), pos)?;
                        self.out.droppable = false;
                    }
                    self.out.place(taken);
                    self.nodes(then)?;
                    self.out.place(join);
                }
            }
            Stmt::Inlined { name, body } => {
                let outer = std::mem::replace(&mut self.out.prefix, name.chars().take(1).collect());
                self.nodes(body)?;
                self.out.prefix = outer;
            }
            Stmt::Label(label) => self.out.place(*label),
            Stmt::Prim(prim) => self.prim(prim, pos)?,
            Stmt::Repeat { label, body } => {
                let label = match label {
                    Some(label) => *label,
                    None => self
                        .out
                        .labels
                        .add(LabelName::Hint(format!("{}loop", self.out.prefix))),
                };
                self.out.place(label);
                self.nodes(body)?;
                self.prim(&Prim::LoopBack(label), pos)?;
            }
        }
        if let Some(text) = self.out.comment.take() {
            self.out.lines.push(Line::Comment(text));
        }
        Ok(())
    }

    fn nodes(&mut self, nodes: &[Node]) -> Result<(), Diagnostic> {
        nodes.iter().try_for_each(|n| self.node(n))
    }

    fn prim(&mut self, prim: &Prim, pos: Pos) -> Result<(), Diagnostic> {
        let start = self.out.lines.len();
        self.backend
            .lower(prim, &mut self.out)
            .map_err(|message| Diagnostic::error(pos, message))?;
        if matches!(prim, Prim::Jump(_) | Prim::Return) {
            self.out.mark_stop(start);
        }
        Ok(())
    }
}

/// Lowers the program to lines. Labels are not named yet.
pub(crate) fn emit(program: &Program, backend: &dyn Backend) -> Result<Emitter, Diagnostic> {
    let mut walker = Walker {
        backend,
        out: Emitter {
            lines: Vec::new(),
            labels: program.labels.clone(),
            comment: None,
            droppable: false,
            prefix: String::new(),
            pos: Pos::default(),
        },
    };
    let mut previous = None;
    for block in &program.blocks {
        walker.block(block, previous)?;
        previous = Some(block.kind);
    }
    let mut out = walker.out;
    drop_jumps_to_next(&mut out.lines);
    Ok(out)
}

/// Removes generated jumps whose target is the next instruction.
fn drop_jumps_to_next(lines: &mut Vec<Line>) {
    let mut index = 0;
    while index < lines.len() {
        let target = match &lines[index] {
            Line::Instr(Instr {
                droppable: true,
                operands,
                ..
            }) => operands.iter().find_map(|o| match o {
                Operand::Label(id) => Some(*id),
                Operand::Text(_) => None,
            }),
            _ => None,
        };
        let lands_next = target.is_some_and(|target| {
            lines[index + 1..]
                .iter()
                .take_while(|l| !matches!(l, Line::Instr(_) | Line::Raw(..)))
                .any(|l| *l == Line::Label(target))
        });
        if lands_next {
            lines.remove(index);
        } else {
            index += 1;
        }
    }
}
