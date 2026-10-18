use ld60_calco::datasheet::{INSTRUCTIONS, PORTS};
use ld60_calco::{Addr, Cond as Jcc, Op, RAM_LEN, Revision};
use ld60_machine::Isa;

use crate::backend::{Backend, Features, Port};
use crate::emit::{Emitter, Instr, Operand};
use crate::hir::{CmpOp, Cond, Place, Prim, Value};
use crate::labels::Role;

const INTRINSICS: &[&str] = &["da", "ds", "nop"];
const STACK_DEPTH: u8 = 2;

pub(crate) struct CalcoBackend {
    revision: Revision,
    features: Features,
    ports: Vec<Port>,
}

impl CalcoBackend {
    pub(crate) fn new(revision: Revision) -> Self {
        let has = |op| revision.supports(op);
        let features = Features {
            calls: has(Op::Call(0)) && has(Op::Ret),
            loops: has(Op::Djnz(0)) && has(Op::LdiB(0)),
            indexed: has(Op::Ld(Addr::Indexed(0))) && has(Op::St(Addr::Indexed(0))),
            ram_cells: RAM_LEN as u8,
            stack_depth: Some(STACK_DEPTH),
        };
        let ports = PORTS
            .iter()
            .map(|doc| Port {
                name: doc.name,
                output: doc.output,
            })
            .collect();
        Self {
            revision,
            features,
            ports,
        }
    }

    fn branch(&self, cond: &Cond, target: Operand, out: &mut Emitter) -> Result<(), String> {
        let jcc = match cond {
            Cond::Carry(true) => Jcc::Carry,
            Cond::Carry(false) => Jcc::NoCarry,
            Cond::Zero(true) => Jcc::Zero,
            Cond::Zero(false) => Jcc::NotZero,
            Cond::Compare(op, value) => {
                match value {
                    Value::Imm(literal) => self.emit(
                        out,
                        Op::Cpi(literal.value),
                        vec![Operand::Text(literal.text.clone())],
                    )?,
                    Value::Place(place) => {
                        let (addr, operand) = memory(*place)?;
                        self.emit(out, Op::Cmp(addr), vec![operand])?;
                    }
                }
                match op {
                    CmpOp::Eq => Jcc::Zero,
                    CmpOp::Ne => Jcc::NotZero,
                    CmpOp::Lt => Jcc::Carry,
                    CmpOp::Ge => Jcc::NoCarry,
                }
            }
        };
        let inverse = match jcc {
            Jcc::Always => Jcc::Always,
            Jcc::Zero => Jcc::NotZero,
            Jcc::NotZero => Jcc::Zero,
            Jcc::Carry => Jcc::NoCarry,
            Jcc::NoCarry => Jcc::Carry,
        };
        let supports = |c| self.revision.supports(Op::Jump(c, 0));
        if !supports(jcc) && supports(inverse) && supports(Jcc::Always) {
            let skip = out.join();
            self.emit(out, Op::Jump(inverse, 0), vec![Operand::Label(skip)])?;
            self.emit(out, Op::Jump(Jcc::Always, 0), vec![target])?;
            out.place(skip);
            return Ok(());
        }
        self.emit(out, Op::Jump(jcc, 0), vec![target])
    }

    fn emit(&self, out: &mut Emitter, op: Op, operands: Vec<Operand>) -> Result<(), String> {
        if !self.revision.supports(op) {
            return Err(format!(
                "'{}' is not supported by {}",
                form(op),
                self.name()
            ));
        }
        out.op(mnemonic(op), operands);
        Ok(())
    }

    fn port(&self, name: &str, output: bool) -> Result<u8, String> {
        PORTS
            .iter()
            .find(|doc| doc.name == name && doc.output == output)
            .map(|doc| doc.port)
            .ok_or_else(|| format!("{} has no port {name}", self.name()))
    }
}

impl Backend for CalcoBackend {
    fn features(&self) -> &Features {
        &self.features
    }

    fn intrinsics(&self) -> &[&'static str] {
        INTRINSICS
    }

    fn label_role(&self, instr: &Instr) -> Role {
        match instr.mnemonic {
            "ST" => Role::Word("store"),
            "LD" => Role::Word("load"),
            "OUT" => Role::Word("out"),
            "CALL" => match instr.operands.first() {
                Some(Operand::Label(id)) => Role::Target(*id),
                _ => Role::Word("skip"),
            },
            _ => Role::Word("skip"),
        }
    }

    fn lower(&self, prim: &Prim, out: &mut Emitter) -> Result<(), String> {
        let label = |id| vec![Operand::Label(id)];
        match prim {
            Prim::AddCarry(value) | Prim::SubCarry(value) => {
                let Value::Place(place) = value else {
                    return Err("ADC and SBC need a cell, not a number".into());
                };
                let (addr, operand) = memory(*place)?;
                let op = match prim {
                    Prim::AddCarry(_) => Op::Adc(addr),
                    _ => Op::Sbc(addr),
                };
                self.emit(out, op, vec![operand])
            }
            Prim::Branch(cond, target) => self.branch(cond, Operand::Label(*target), out),
            Prim::Call(target) => self.emit(out, Op::Call(0), label(*target)),
            Prim::Dec => self.emit(out, Op::Dec, Vec::new()),
            Prim::Inc => self.emit(out, Op::Inc, Vec::new()),
            Prim::Intrinsic(name) => {
                let op = match *name {
                    "da" => Op::Da,
                    "ds" => Op::Ds,
                    _ => Op::Nop,
                };
                self.emit(out, op, Vec::new())
            }
            Prim::Jump(target) => self.emit(out, Op::Jump(Jcc::Always, 0), label(*target)),
            Prim::Load(Value::Imm(literal)) => self.emit(
                out,
                Op::LdiA(literal.value),
                vec![text("A"), text(&literal.text)],
            ),
            Prim::Load(Value::Place(Place::Port(name))) => {
                let port = self.port(name, false)?;
                self.emit(out, Op::In(port), vec![text(name)])
            }
            Prim::Load(Value::Place(place)) => {
                let (addr, operand) = memory(*place)?;
                self.emit(out, Op::Ld(addr), vec![operand])
            }
            Prim::LoopBack(head) => self.emit(out, Op::Djnz(0), label(*head)),
            Prim::Move { dst, src } => {
                self.lower(&Prim::Load(src.clone()), out)?;
                self.lower(&Prim::Store(*dst), out)
            }
            Prim::Return => self.emit(out, Op::Ret, Vec::new()),
            Prim::SetCarry(true) => self.emit(out, Op::Sec, Vec::new()),
            Prim::SetCarry(false) => self.emit(out, Op::Clc, Vec::new()),
            Prim::SetIndex(literal) => self.emit(
                out,
                Op::LdiB(literal.value),
                vec![text("B"), text(&literal.text)],
            ),
            Prim::Store(Place::Port(name)) => {
                let port = self.port(name, true)?;
                self.emit(out, Op::Out(port), vec![text(name)])
            }
            Prim::Store(place) => {
                let (addr, operand) = memory(*place)?;
                self.emit(out, Op::St(addr), vec![operand])
            }
            Prim::Wait => self.emit(out, Op::Hlt, Vec::new()),
        }
    }

    fn name(&self) -> &'static str {
        self.revision.isa().name()
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }
}

/// The datasheet form of `op`, like `LD [B+n]` or `JNZ aa`.
fn form(op: Op) -> &'static str {
    if let Op::Jump(cond, _) = op {
        return cond.form();
    }
    let shape = |op: Op| match op {
        Op::Ld(addr) => Op::Ld(shape_addr(addr)),
        Op::St(addr) => Op::St(shape_addr(addr)),
        Op::Adc(addr) => Op::Adc(shape_addr(addr)),
        Op::Sbc(addr) => Op::Sbc(shape_addr(addr)),
        Op::Cmp(addr) => Op::Cmp(shape_addr(addr)),
        Op::LdiA(_) => Op::LdiA(0),
        Op::LdiB(_) => Op::LdiB(0),
        Op::Cpi(_) => Op::Cpi(0),
        Op::In(_) => Op::In(0),
        Op::Out(_) => Op::Out(0),
        Op::Call(_) => Op::Call(0),
        Op::Djnz(_) => Op::Djnz(0),
        op => op,
    };
    INSTRUCTIONS
        .iter()
        .find(|doc| shape(doc.op) == shape(op))
        .map_or("?", |doc| doc.syntax)
}

fn memory(place: Place) -> Result<(Addr, Operand), String> {
    match place {
        Place::Cell(n) => Ok((Addr::Direct(n), text(&format!("M{n:X}")))),
        Place::Indexed(0) => Ok((Addr::Indexed(0), text("[B]"))),
        Place::Indexed(n) => Ok((Addr::Indexed(n), text(&format!("[B+{n}]")))),
        Place::Port(name) => Err(format!("port {name} is not a RAM cell")),
    }
}

fn mnemonic(op: Op) -> &'static str {
    match op {
        Op::Nop | Op::Reserved(_) => "NOP",
        Op::Hlt => "HLT",
        Op::Ret => "RET",
        Op::Swp => "SWP",
        Op::Clc => "CLC",
        Op::Sec => "SEC",
        Op::Inc => "INC",
        Op::Dec => "DEC",
        Op::Da => "DA",
        Op::Ds => "DS",
        Op::Ld(_) => "LD",
        Op::St(_) => "ST",
        Op::Adc(_) => "ADC",
        Op::Sbc(_) => "SBC",
        Op::Cmp(_) => "CMP",
        Op::LdiA(_) | Op::LdiB(_) => "LDI",
        Op::Cpi(_) => "CPI",
        Op::In(_) => "IN",
        Op::Out(_) => "OUT",
        Op::Jump(cond, _) => cond.mnemonic(),
        Op::Call(_) => "CALL",
        Op::Djnz(_) => "DJNZ",
    }
}

fn shape_addr(addr: Addr) -> Addr {
    match addr {
        Addr::Direct(_) => Addr::Direct(0),
        Addr::Indexed(_) => Addr::Indexed(0),
    }
}

fn text(text: &str) -> Operand {
    Operand::Text(text.into())
}
