use std::collections::BTreeMap;

use crate::ast::{self, AssignOp, BinOp, BlockKind, ExprKind, File, Item, StmtKind};
use crate::diagnostic::Pos;
use crate::hir::{Block, CmpOp, Cond, Node, Place, Prim, Program, RamLine, Stmt, Value};
use crate::labels::{LabelId, LabelName, Labels};
use crate::literal::Literal;
use crate::{Diagnostic, Target};

const RESERVED: &[&str] = &["a", "i", "carry", "zero", "bcd_add", "bcd_sub"];

type Result<T> = std::result::Result<T, Diagnostic>;

#[derive(Clone, Copy, Debug)]
struct Cell {
    addr: u8,
    len: Option<u8>,
}

struct Routine {
    label: LabelId,
    params: Vec<String>,
}

struct Resolver<'a> {
    target: &'a Target,
    cells: BTreeMap<String, Cell>,
    consts: BTreeMap<String, Literal>,
    labels: BTreeMap<String, LabelId>,
    routines: BTreeMap<String, Routine>,
    /// The block each label is defined in.
    owners: BTreeMap<String, String>,
    /// The block being resolved.
    current: String,
    table: Labels,
    errors: Vec<Diagnostic>,
}

impl Resolver<'_> {
    fn a_value(&self, value: &ast::Expr) -> Result<Vec<Prim>> {
        let load = |expr: &ast::Expr| -> Result<Vec<Prim>> {
            if is_name(expr, "a") {
                Ok(Vec::new())
            } else {
                Ok(vec![Prim::Load(self.value(expr)?)])
            }
        };
        match &value.kind {
            ExprKind::Binary(op, x, one) if is_one(one) => {
                let mut prims = load(x)?;
                prims.push(match op {
                    BinOp::Add => Prim::Inc,
                    BinOp::Sub => Prim::Dec,
                });
                Ok(prims)
            }
            ExprKind::Binary(op, inner, carry) if is_name(carry, "carry") => {
                let ExprKind::Binary(inner_op, x, y) = &inner.kind else {
                    return Err(Diagnostic::error(
                        value.pos,
                        "expected 'x + y + carry' or 'x - y - carry'",
                    ));
                };
                if inner_op != op {
                    return Err(Diagnostic::error(
                        value.pos,
                        "expected 'x + y + carry' or 'x - y - carry'",
                    ));
                }
                let mut prims = load(x)?;
                let y = self.value(y)?;
                prims.push(match op {
                    BinOp::Add => Prim::AddCarry(y),
                    BinOp::Sub => Prim::SubCarry(y),
                });
                Ok(prims)
            }
            ExprKind::Call(name, args) if name == "bcd_add" || name == "bcd_sub" => {
                let [x, y] = args.as_slice() else {
                    return Err(Diagnostic::error(
                        value.pos,
                        format!("{name} takes 2 values"),
                    ));
                };
                let adjust = if name == "bcd_add" { "da" } else { "ds" };
                let adjust = self.intrinsic(adjust).ok_or_else(|| {
                    Diagnostic::error(
                        value.pos,
                        format!(
                            "{name} needs {adjust}(), which {} lacks",
                            self.backend_name()
                        ),
                    )
                })?;
                let mut prims = load(x)?;
                let y = self.value(y)?;
                prims.push(if name == "bcd_add" {
                    Prim::AddCarry(y)
                } else {
                    Prim::SubCarry(y)
                });
                prims.push(Prim::Intrinsic(adjust));
                Ok(prims)
            }
            _ => load(value),
        }
    }

    fn assign(&self, target: &ast::Expr, op: AssignOp, value: &ast::Expr) -> Result<Vec<Prim>> {
        let pos = target.pos;
        if is_name(target, "a") {
            return match op {
                AssignOp::Set => self.a_value(value),
                AssignOp::Add if is_one(value) => Ok(vec![Prim::Inc]),
                AssignOp::Sub if is_one(value) => Ok(vec![Prim::Dec]),
                _ => Err(Diagnostic::error(pos, "only 'a += 1' and 'a -= 1' exist")),
            };
        }
        if op != AssignOp::Set {
            return Err(Diagnostic::error(pos, "only 'a' supports '+=' and '-='"));
        }
        if is_name(target, "i") {
            return match self.value(value)? {
                Value::Imm(literal) => Ok(vec![Prim::SetIndex(literal)]),
                Value::Place(_) => Err(Diagnostic::error(
                    value.pos,
                    "i can only be set to a number",
                )),
            };
        }
        if is_name(target, "carry") {
            return match self.value(value)? {
                Value::Imm(literal) if literal.value <= 1 => {
                    Ok(vec![Prim::SetCarry(literal.value == 1)])
                }
                _ => Err(Diagnostic::error(value.pos, "carry can only be 0 or 1")),
            };
        }
        let dst = self.place(target, true)?;
        if is_name(value, "a") {
            return Ok(vec![Prim::Store(dst)]);
        }
        Ok(vec![Prim::Move {
            dst,
            src: self.value(value)?,
        }])
    }

    fn backend_name(&self) -> &'static str {
        self.target.backend.name()
    }

    fn block(&mut self, block: &ast::Block) -> Block {
        self.current.clone_from(&block.name);
        let body = self.stmts(&block.body, block.kind);
        Block {
            kind: block.kind,
            name: block.name.clone(),
            label: self.labels[&block.name],
            body,
            pos: block.pos,
        }
    }

    fn cond(&self, cond: &ast::Cond) -> Result<Cond> {
        match cond {
            ast::Cond::Flag { name, set, pos } => match name.as_str() {
                "carry" => Ok(Cond::Carry(*set)),
                "zero" => Ok(Cond::Zero(*set)),
                _ => Err(Diagnostic::error(
                    *pos,
                    format!("expected 'carry' or 'zero', found '{name}'"),
                )),
            },
            ast::Cond::Compare { lhs, op, rhs } => {
                if !is_name(lhs, "a") {
                    return Err(Diagnostic::error(lhs.pos, "only 'a' can be compared"));
                }
                let op = match op {
                    ast::CmpOp::Eq => CmpOp::Eq,
                    ast::CmpOp::Ge => CmpOp::Ge,
                    ast::CmpOp::Lt => CmpOp::Lt,
                    ast::CmpOp::Ne => CmpOp::Ne,
                };
                Ok(Cond::Compare(op, self.value(rhs)?))
            }
        }
    }

    fn declare_label(&mut self, name: &str, pos: Pos) {
        if self.labels.contains_key(name) {
            self.errors.push(Diagnostic::error(
                pos,
                format!("label '{name}' is defined twice"),
            ));
            return;
        }
        if let Err(error) = self.check_free(name, pos) {
            self.errors.push(error);
            return;
        }
        let id = self.table.add(LabelName::Fixed(name.into()));
        self.labels.insert(name.into(), id);
        self.owners.insert(name.into(), self.current.clone());
    }

    fn declare_labels(&mut self, stmts: &[ast::Stmt]) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Label(name) => self.declare_label(name, stmt.pos),
                StmtKind::For { label, body, .. } | StmtKind::Repeat { label, body, .. } => {
                    if let Some(label) = label {
                        self.declare_label(label, stmt.pos);
                    }
                    self.declare_labels(body);
                }
                StmtKind::If { then, els, .. } => {
                    self.declare_labels(then);
                    self.declare_labels(els);
                }
                _ => {}
            }
        }
    }

    fn check_free(&self, name: &str, pos: Pos) -> Result<()> {
        let backend = &self.target.backend;
        let taken = RESERVED.contains(&name)
            || backend.ports().iter().any(|p| p.name == name)
            || backend.intrinsics().contains(&name)
            || self.cells.contains_key(name)
            || self.consts.contains_key(name);
        if taken {
            return Err(Diagnostic::error(pos, format!("'{name}' is already used")));
        }
        Ok(())
    }

    fn intrinsic(&self, name: &str) -> Option<&'static str> {
        self.target
            .backend
            .intrinsics()
            .iter()
            .find(|&&i| i == name)
            .copied()
    }

    /// A label that the current block may jump to. Routines are only entered by a call.
    fn jump_target(&self, name: &str, pos: Pos) -> Result<LabelId> {
        let id = self.label(name, pos)?;
        match self.owners.get(name) {
            Some(owner) if *owner != self.current && self.routines.contains_key(owner) => {
                let message = if owner == name {
                    format!("{name} is a routine. Call it with {name}()")
                } else {
                    format!("'{name}' is inside routine {owner}. Only {owner} can jump to it")
                };
                Err(Diagnostic::error(pos, message))
            }
            _ => Ok(id),
        }
    }

    fn label(&self, name: &str, pos: Pos) -> Result<LabelId> {
        self.labels
            .get(name)
            .copied()
            .ok_or_else(|| Diagnostic::error(pos, format!("unknown label '{name}'")))
    }

    fn literal(&self, expr: &ast::Expr) -> Result<Literal> {
        match self.value(expr)? {
            Value::Imm(literal) => Ok(literal),
            Value::Place(_) => Err(Diagnostic::error(expr.pos, "expected a number")),
        }
    }

    fn pattern(&self, pattern: &ast::Expr) -> Result<Cond> {
        let ExprKind::Range(lo, hi) = &pattern.kind else {
            return Ok(Cond::Compare(CmpOp::Eq, self.value(pattern)?));
        };
        if self.literal(lo)?.value != 0 {
            return Err(Diagnostic::error(lo.pos, "ranges must start at 0"));
        }
        let hi = self.literal(hi)?;
        let end = hi
            .value
            .checked_add(1)
            .ok_or_else(|| Diagnostic::error(pattern.pos, "range end must be below 255"))?;
        Ok(Cond::Compare(CmpOp::Lt, Value::Imm(hi.with_value(end))))
    }

    fn place(&self, expr: &ast::Expr, output: bool) -> Result<Place> {
        let pos = expr.pos;
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(port) = self.target.backend.ports().iter().find(|p| p.name == name) {
                    if port.output != output {
                        let use_ = if output { "written" } else { "read" };
                        return Err(Diagnostic::error(
                            pos,
                            format!("port {name} cannot be {use_}"),
                        ));
                    }
                    return Ok(Place::Port(port.name));
                }
                match self.cells.get(name) {
                    Some(Cell { addr, len: None }) => Ok(Place::Cell(*addr)),
                    Some(_) => Err(Diagnostic::error(
                        pos,
                        format!("'{name}' is an array, use {name}[i] or {name}[n]"),
                    )),
                    None => Err(Diagnostic::error(pos, format!("unknown cell '{name}'"))),
                }
            }
            ExprKind::Index(name, index) => {
                let Some(&Cell {
                    addr,
                    len: Some(len),
                }) = self.cells.get(name)
                else {
                    return Err(Diagnostic::error(pos, format!("'{name}' is not an array")));
                };
                if is_name(index, "i") {
                    let cells = self.target.backend.features().ram_cells;
                    return Ok(Place::Indexed((addr + cells - 1) % cells));
                }
                let n = self.literal(index)?.value;
                if !(1..=len).contains(&n) {
                    return Err(Diagnostic::error(
                        index.pos,
                        format!("{name} has elements 1 to {len}"),
                    ));
                }
                Ok(Place::Cell(addr + n - 1))
            }
            _ => Err(Diagnostic::error(pos, "expected a cell or a port")),
        }
    }

    fn stmt(&mut self, stmt: &ast::Stmt, kind: BlockKind) -> Result<Vec<Node>> {
        let pos = stmt.pos;
        let prims = |prims: Vec<Prim>| prims.into_iter().map(Stmt::Prim).collect();
        let stmts: Vec<Stmt> = match &stmt.kind {
            StmtKind::Asm(lines) => vec![Stmt::Asm(lines.clone())],
            StmtKind::Assign { target, op, value } => prims(self.assign(target, *op, value)?),
            StmtKind::Call { name, args } => vec![Stmt::Prim(self.call(name, args, pos)?)],
            StmtKind::Comment(text) => vec![Stmt::Comment(text.clone())],
            StmtKind::For {
                counter,
                start,
                end,
                label,
                body,
            } => {
                check_counter(counter, pos)?;
                if self.literal(end)?.value != 1 {
                    return Err(Diagnostic::error(end.pos, "loops count down to 1"));
                }
                let start = self.literal(start)?;
                vec![
                    Stmt::Prim(Prim::SetIndex(start)),
                    Stmt::Repeat {
                        label: label.as_ref().map(|l| self.label(l, pos)).transpose()?,
                        body: self.stmts(body, kind),
                    },
                ]
            }
            StmtKind::Goto(name) => vec![Stmt::Prim(Prim::Jump(self.jump_target(name, pos)?))],
            StmtKind::If { cond, then, els } => vec![Stmt::If {
                cond: self.cond(cond)?,
                then: self.stmts(then, kind),
                els: self.stmts(els, kind),
            }],
            StmtKind::IfGoto { cond, target } => vec![Stmt::Prim(Prim::Branch(
                self.cond(cond)?,
                self.jump_target(target, pos)?,
            ))],
            StmtKind::Label(name) => vec![Stmt::Label(self.label(name, pos)?)],
            StmtKind::Match { scrutinee, arms } => {
                if !is_name(scrutinee, "a") {
                    return Err(Diagnostic::error(scrutinee.pos, "only 'a' can be matched"));
                }
                let mut nodes = Vec::new();
                for arm in arms {
                    let cond = self.pattern(&arm.pattern)?;
                    let target = self.jump_target(&arm.target, arm.pos)?;
                    nodes.push(Node {
                        stmt: Stmt::Prim(Prim::Branch(cond, target)),
                        comment: arm.comment.clone(),
                        pos: arm.pos,
                    });
                }
                if let (Some(first), Some(comment)) = (nodes.first_mut(), &stmt.comment) {
                    first.comment.get_or_insert_with(|| comment.clone());
                }
                return Ok(nodes);
            }
            StmtKind::Repeat {
                counter,
                label,
                body,
            } => {
                check_counter(counter, pos)?;
                vec![Stmt::Repeat {
                    label: label.as_ref().map(|l| self.label(l, pos)).transpose()?,
                    body: self.stmts(body, kind),
                }]
            }
            StmtKind::Return if kind == BlockKind::Routine => vec![Stmt::Prim(Prim::Return)],
            StmtKind::Return => {
                return Err(Diagnostic::error(
                    pos,
                    "return is only allowed in a routine",
                ));
            }
            StmtKind::Switch(_) => unreachable!("switches are resolved first"),
            StmtKind::Wait => vec![Stmt::Prim(Prim::Wait)],
        };
        let mut nodes: Vec<Node> = stmts
            .into_iter()
            .map(|stmt| Node {
                stmt,
                comment: None,
                pos,
            })
            .collect();
        if let Some(first) = nodes.first_mut() {
            first.comment.clone_from(&stmt.comment);
        }
        Ok(nodes)
    }

    fn call(&self, name: &str, args: &[ast::Expr], pos: Pos) -> Result<Prim> {
        if let Some(routine) = self.routines.get(name) {
            let names: Vec<_> = args
                .iter()
                .map(|a| match &a.kind {
                    ExprKind::Name(n) => Some(n.as_str()),
                    _ => None,
                })
                .collect();
            let expected: Vec<_> = routine.params.iter().map(|p| Some(p.as_str())).collect();
            if names != expected {
                return Err(Diagnostic::error(
                    pos,
                    format!("{name} expects ({})", routine.params.join(", ")),
                ));
            }
            return Ok(Prim::Call(routine.label));
        }
        if let Some(intrinsic) = self.intrinsic(name) {
            if !args.is_empty() {
                return Err(Diagnostic::error(pos, format!("{name}() takes no values")));
            }
            return Ok(Prim::Intrinsic(intrinsic));
        }
        Err(Diagnostic::error(pos, format!("unknown routine '{name}'")))
    }

    fn stmts(&mut self, stmts: &[ast::Stmt], kind: BlockKind) -> Vec<Node> {
        let mut nodes = Vec::new();
        for stmt in stmts {
            match self.stmt(stmt, kind) {
                Ok(more) => nodes.extend(more),
                Err(error) => self.errors.push(error),
            }
        }
        nodes
    }

    fn value(&self, expr: &ast::Expr) -> Result<Value> {
        let pos = expr.pos;
        match &expr.kind {
            ExprKind::Num(literal) => Ok(Value::Imm(literal.clone())),
            ExprKind::Key(key) => self
                .target
                .key_code(*key)
                .map(|code| Value::Imm(Literal::hex(code)))
                .ok_or_else(|| {
                    Diagnostic::error(
                        pos,
                        format!("{} has no key '{key}'", self.target.device_name()),
                    )
                }),
            ExprKind::Name(name) if self.consts.contains_key(name) => {
                Ok(Value::Imm(self.consts[name].clone()))
            }
            ExprKind::Name(_) | ExprKind::Index(..) => self.place(expr, false).map(Value::Place),
            ExprKind::Range(..) => Err(Diagnostic::error(pos, "ranges are only allowed in match")),
            _ => Err(Diagnostic::error(
                pos,
                "expected a number, a cell or a port",
            )),
        }
    }
}

fn check_counter(counter: &str, pos: Pos) -> Result<()> {
    if counter == "i" {
        Ok(())
    } else {
        Err(Diagnostic::error(pos, "the loop counter is always 'i'"))
    }
}

fn is_name(expr: &ast::Expr, name: &str) -> bool {
    matches!(&expr.kind, ExprKind::Name(n) if n == name)
}

fn is_one(expr: &ast::Expr) -> bool {
    matches!(&expr.kind, ExprKind::Num(literal) if literal.value == 1)
}

fn parse_cell(addr: &str) -> Option<u8> {
    let digits = addr.strip_prefix('M')?;
    if digits.is_empty() || digits.len() > 2 {
        return None;
    }
    u8::from_str_radix(digits, 16).ok()
}

/// Turns the switch-free syntax tree into the program for `target`.
pub(crate) fn resolve(
    file: &File,
    target: &Target,
) -> std::result::Result<Program, Vec<Diagnostic>> {
    let mut resolver = Resolver {
        target,
        cells: BTreeMap::new(),
        consts: BTreeMap::new(),
        labels: BTreeMap::new(),
        routines: BTreeMap::new(),
        owners: BTreeMap::new(),
        current: String::new(),
        table: Labels::default(),
        errors: Vec::new(),
    };
    let mut title = None;
    let mut ram = Vec::new();
    let mut notes = Vec::new();
    let mut blocks = Vec::new();
    let ram_cells = target.backend.features().ram_cells;
    for item in &file.items {
        match item {
            Item::Firmware(decl) => {
                if title.is_some() {
                    resolver
                        .errors
                        .push(Diagnostic::error(decl.pos, "firmware is declared twice"));
                }
                if decl.device != target.device_name() {
                    resolver.errors.push(Diagnostic::error(
                        decl.pos,
                        format!(
                            "firmware is for {}, but the target is {}",
                            decl.device,
                            target.device_name()
                        ),
                    ));
                }
                title = Some(decl.title.clone());
            }
            Item::Cell(decl) => {
                let len = decl.len.unwrap_or(1);
                let addr = parse_cell(&decl.addr)
                    .filter(|&a| len > 0 && u16::from(a) + u16::from(len) <= u16::from(ram_cells));
                let Some(addr) = addr else {
                    resolver.errors.push(Diagnostic::error(
                        decl.pos,
                        format!("'{}' does not fit in RAM", decl.addr),
                    ));
                    continue;
                };
                if let Err(error) = resolver.check_free(&decl.name, decl.pos) {
                    resolver.errors.push(error);
                    continue;
                }
                let last = addr + len - 1;
                if let Some(other) = ram
                    .iter()
                    .find(|l: &&RamLine| l.first <= last && addr <= l.last)
                {
                    resolver.errors.push(Diagnostic::error(
                        decl.pos,
                        format!("'{}' overlaps the cell at M{:X}", decl.name, other.first),
                    ));
                    continue;
                }
                resolver.cells.insert(
                    decl.name.clone(),
                    Cell {
                        addr,
                        len: decl.len,
                    },
                );
                ram.push(RamLine {
                    first: addr,
                    last,
                    text: decl.comment.clone().unwrap_or_else(|| decl.name.clone()),
                });
            }
            Item::Const(decl) => match resolver.check_free(&decl.name, decl.pos) {
                Ok(()) => {
                    resolver
                        .consts
                        .insert(decl.name.clone(), decl.value.clone());
                }
                Err(error) => resolver.errors.push(error),
            },
            Item::Comment(text) => notes.push(text.clone()),
            Item::Block(block) => blocks.push(block),
            Item::Switch(_) => unreachable!("switches are resolved first"),
        }
    }
    for block in &blocks {
        resolver.current.clone_from(&block.name);
        resolver.declare_label(&block.name, block.pos);
        resolver.declare_labels(&block.body);
        if block.kind == BlockKind::Routine {
            if let Some(param) = block.params.iter().find(|p| *p != "i") {
                resolver.errors.push(Diagnostic::error(
                    block.pos,
                    format!("routines can only take 'i', found '{param}'"),
                ));
            }
            if let Some(&label) = resolver.labels.get(&block.name) {
                resolver.routines.insert(
                    block.name.clone(),
                    Routine {
                        label,
                        params: block.params.clone(),
                    },
                );
            }
        }
    }
    match blocks.iter().position(|b| b.kind == BlockKind::Entry) {
        Some(0) => {}
        Some(n) => resolver.errors.push(Diagnostic::error(
            blocks[n].pos,
            "the entry block must come first",
        )),
        None => resolver
            .errors
            .push(Diagnostic::error(Pos::default(), "there is no entry block")),
    }
    if let Some(extra) = blocks.iter().filter(|b| b.kind == BlockKind::Entry).nth(1) {
        resolver.errors.push(Diagnostic::error(
            extra.pos,
            "there is more than one entry block",
        ));
    }
    blocks.retain(|b| resolver.labels.contains_key(&b.name));
    let blocks: Vec<Block> = blocks.into_iter().map(|b| resolver.block(b)).collect();
    for pair in blocks.windows(2) {
        let [block, routine] = pair else { continue };
        if block.kind != BlockKind::Routine
            && routine.kind == BlockKind::Routine
            && !Node::ends_in_transfer(&block.body)
        {
            resolver.errors.push(Diagnostic::error(
                routine.pos,
                format!(
                    "{} falls through into routine {}. End {} with a goto",
                    block.name, routine.name, block.name
                ),
            ));
        }
    }
    let Some(title) = title else {
        resolver.errors.push(Diagnostic::error(
            Pos::default(),
            "missing 'firmware \"title\" for device'",
        ));
        return Err(resolver.errors);
    };
    if !resolver.errors.is_empty() {
        return Err(resolver.errors);
    }
    Ok(Program {
        title,
        ram,
        notes,
        blocks,
        labels: resolver.table,
    })
}
