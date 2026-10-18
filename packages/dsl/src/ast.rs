use crate::diagnostic::Pos;
use crate::literal::Literal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct File {
    pub(crate) items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    Block(Block),
    Cell(CellDecl),
    Comment(String),
    Const(ConstDecl),
    Firmware(FirmwareDecl),
    Switch(Switch<Item>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FirmwareDecl {
    pub(crate) title: String,
    pub(crate) device: String,
    pub(crate) pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CellDecl {
    pub(crate) name: String,
    /// Some for arrays.
    pub(crate) len: Option<u8>,
    pub(crate) addr: String,
    pub(crate) comment: Option<String>,
    pub(crate) pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConstDecl {
    pub(crate) name: String,
    pub(crate) value: Literal,
    pub(crate) pos: Pos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockKind {
    Block,
    Entry,
    Routine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Block {
    pub(crate) kind: BlockKind,
    pub(crate) name: String,
    pub(crate) params: Vec<String>,
    pub(crate) body: Vec<Stmt>,
    pub(crate) pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stmt {
    pub(crate) kind: StmtKind,
    /// Trailing `;` comment.
    pub(crate) comment: Option<String>,
    pub(crate) pos: Pos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssignOp {
    Add,
    Set,
    Sub,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StmtKind {
    Asm(Vec<String>),
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Comment(String),
    For {
        counter: String,
        start: Expr,
        end: Expr,
        label: Option<String>,
        body: Vec<Stmt>,
    },
    Goto(String),
    If {
        cond: Cond,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
    },
    IfGoto {
        cond: Cond,
        target: String,
    },
    Label(String),
    Match {
        scrutinee: Expr,
        arms: Vec<Arm>,
    },
    Repeat {
        counter: String,
        label: Option<String>,
        body: Vec<Stmt>,
    },
    Return,
    Switch(Switch<Stmt>),
    Wait,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Arm {
    pub(crate) pattern: Expr,
    pub(crate) target: String,
    pub(crate) comment: Option<String>,
    pub(crate) pos: Pos,
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
    Compare { lhs: Expr, op: CmpOp, rhs: Expr },
    Flag { name: String, set: bool, pos: Pos },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add,
    Sub,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Expr {
    pub(crate) kind: ExprKind,
    pub(crate) pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExprKind {
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Index(String, Box<Expr>),
    Key(char),
    Name(String),
    Num(Literal),
    /// `lo to hi`, only in match patterns.
    Range(Box<Expr>, Box<Expr>),
    Switch(Box<Switch<Expr>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SwitchKind {
    /// Selected: the buggy firmware takes the `buggy` side.
    Bug,
    /// Selected: both firmware files take the `after` side.
    Change,
    /// Selected: the fixed firmware takes the `after` side.
    Fix,
}

impl SwitchKind {
    /// Names of the default and the other side.
    pub(crate) fn sides(self) -> [&'static str; 2] {
        match self {
            Self::Bug => ["fixed", "buggy"],
            Self::Change | Self::Fix => ["before", "after"],
        }
    }

    pub(crate) fn from_keyword(word: &str) -> Option<Self> {
        match word {
            "bug" => Some(Self::Bug),
            "change" => Some(Self::Change),
            "fix" => Some(Self::Fix),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Switch<T> {
    pub(crate) kind: SwitchKind,
    pub(crate) name: String,
    pub(crate) default: Vec<T>,
    pub(crate) other: Vec<T>,
    pub(crate) pos: Pos,
}
