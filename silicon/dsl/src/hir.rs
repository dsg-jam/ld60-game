use crate::Diagnostic;
use crate::ast::BlockKind;
use crate::diagnostic::Pos;
use crate::labels::{LabelId, Labels};
use crate::literal::Literal;

#[derive(Clone, Debug)]
pub(crate) struct Program {
    pub(crate) title: String,
    /// RAM map lines for the header, in declaration order.
    pub(crate) ram: Vec<RamLine>,
    /// Top-level `;` comments for the header.
    pub(crate) notes: Vec<String>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) labels: Labels,
    pub(crate) warnings: Vec<Diagnostic>,
}

impl Program {
    pub(crate) fn routine(&self, label: LabelId) -> Option<&Block> {
        self.blocks
            .iter()
            .find(|b| b.label == label && b.kind == BlockKind::Routine)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RamLine {
    pub(crate) first: u8,
    pub(crate) last: u8,
    pub(crate) text: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Block {
    pub(crate) kind: BlockKind,
    pub(crate) name: String,
    pub(crate) label: LabelId,
    pub(crate) body: Vec<Node>,
    pub(crate) pos: Pos,
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) stmt: Stmt,
    pub(crate) comment: Option<String>,
    pub(crate) pos: Pos,
}

impl Node {
    /// Whether control never continues after the last of `nodes`.
    pub(crate) fn ends_in_transfer(nodes: &[Node]) -> bool {
        matches!(
            nodes.last(),
            Some(Node {
                stmt: Stmt::Prim(Prim::Jump(_) | Prim::Return),
                ..
            })
        )
    }

    /// Calls `f` on every place that `nodes` read or write.
    pub(crate) fn places(nodes: &[Node], f: &mut impl FnMut(Place)) {
        let value = |v: &Value, f: &mut dyn FnMut(Place)| {
            if let Value::Place(place) = v {
                f(*place);
            }
        };
        Self::visit(nodes, &mut |node| match &node.stmt {
            Stmt::Prim(prim) => match prim {
                Prim::AddCarry(v) | Prim::Load(v) | Prim::SubCarry(v) => value(v, f),
                Prim::Branch(Cond::Compare(_, v), _) => value(v, f),
                Prim::Move { dst, src } => {
                    f(*dst);
                    value(src, f);
                }
                Prim::Store(place) => f(*place),
                _ => {}
            },
            Stmt::If {
                cond: Cond::Compare(_, v),
                ..
            } => value(v, f),
            _ => {}
        });
    }

    /// Calls `f` on every node, depth first.
    pub(crate) fn visit(nodes: &[Node], f: &mut impl FnMut(&Node)) {
        for node in nodes {
            f(node);
            match &node.stmt {
                Stmt::If { then, els, .. } => {
                    Self::visit(then, f);
                    Self::visit(els, f);
                }
                Stmt::Inlined { body, .. } | Stmt::Repeat { body, .. } => Self::visit(body, f),
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Stmt {
    Asm(Vec<String>),
    Comment(String),
    If {
        cond: Cond,
        then: Vec<Node>,
        els: Vec<Node>,
    },
    /// The body of an inlined routine. Generated labels inside it use its name.
    Inlined {
        name: String,
        body: Vec<Node>,
    },
    Label(LabelId),
    Prim(Prim),
    Repeat {
        label: Option<LabelId>,
        body: Vec<Node>,
    },
}

/// One short instruction pattern of the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Prim {
    /// `a = a + v + carry`
    AddCarry(Value),
    Branch(Cond, LabelId),
    Call(LabelId),
    Dec,
    Inc,
    Intrinsic(&'static str),
    Jump(LabelId),
    Load(Value),
    /// Jumps back to the loop head while the counter is not 0 after the decrement.
    LoopBack(LabelId),
    /// `dst = src` through `a`.
    Move {
        dst: Place,
        src: Value,
    },
    Return,
    SetFlag(&'static str, bool),
    SetIndex(Literal),
    Store(Place),
    /// `a = a - v - carry`
    SubCarry(Value),
    Wait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    Cell(u8),
    /// The cell at `i + offset`.
    Indexed(u8),
    Port(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Imm(Literal),
    Place(Place),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CmpOp {
    Eq,
    Ge,
    Lt,
    Ne,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cond {
    /// `a <op> value`
    Compare(CmpOp, Value),
    /// A backend flag, set or clear.
    Flag(&'static str, bool),
}

impl Cond {
    pub(crate) fn inverse(&self) -> Self {
        match self {
            Self::Compare(op, value) => {
                let op = match op {
                    CmpOp::Eq => CmpOp::Ne,
                    CmpOp::Ne => CmpOp::Eq,
                    CmpOp::Lt => CmpOp::Ge,
                    CmpOp::Ge => CmpOp::Lt,
                };
                Self::Compare(op, value.clone())
            }
            Self::Flag(name, set) => Self::Flag(name, !set),
        }
    }
}
