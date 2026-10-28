const MISC: [Op; 10] = [
    Op::Nop,
    Op::Wait,
    Op::Ret,
    Op::Swp,
    Op::Clc,
    Op::Sec,
    Op::Inc,
    Op::Dec,
    Op::Da,
    Op::Ds,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Addr {
    Direct(u8),
    Indexed(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    Always,
    Zero,
    NotZero,
    Carry,
    NoCarry,
}

impl Cond {
    pub const ALL: [Cond; 5] = [
        Self::Always,
        Self::Zero,
        Self::NotZero,
        Self::Carry,
        Self::NoCarry,
    ];

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(usize::from(code)).copied()
    }

    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Always => "JMP",
            Self::Zero => "JZ",
            Self::NotZero => "JNZ",
            Self::Carry => "JC",
            Self::NoCarry => "JNC",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Nop,
    /// Waits for a new key press. The assembler also takes `WAIT`.
    Wait,
    Ret,
    Swp,
    Clc,
    Sec,
    Inc,
    Dec,
    Da,
    Ds,
    Ld(Addr),
    St(Addr),
    Adc(Addr),
    Sbc(Addr),
    Cmp(Addr),
    LdiA(u8),
    LdiB(u8),
    Cpi(u8),
    In(u8),
    Out(u8),
    Jump(Cond, u8),
    Call(u8),
    Djnz(u8),
    /// Undocumented. Behaves like `NOP`.
    Reserved(u8),
}

impl Op {
    pub fn decode(first: u8, second: u8) -> Self {
        let n = first & 0x0F;
        let direct = Addr::Direct(n);
        let indexed = Addr::Indexed(n);
        match first >> 4 {
            0x0 => MISC
                .get(usize::from(n))
                .copied()
                .unwrap_or(Self::Reserved(first)),
            0x1 => Self::Ld(direct),
            0x2 => Self::St(direct),
            0x3 => match n {
                0 => Self::LdiA(second),
                1 => Self::LdiB(second),
                2 => Self::Cpi(second),
                _ => Self::Reserved(first),
            },
            0x4 => Self::Adc(direct),
            0x5 => Self::Sbc(direct),
            0x6 => Self::Cmp(direct),
            0x7 => Self::In(n),
            0x8 => Self::Out(n),
            0x9 => Self::Ld(indexed),
            0xA => Self::St(indexed),
            0xB => Cond::from_code(n).map_or(Self::Reserved(first), |c| Self::Jump(c, second)),
            0xC => Self::Adc(indexed),
            0xD => Self::Sbc(indexed),
            0xE => Self::Cmp(indexed),
            _ => match n {
                0 => Self::Call(second),
                1 => Self::Djnz(second),
                _ => Self::Reserved(first),
            },
        }
    }

    pub fn size(self) -> u8 {
        match self {
            Self::LdiA(_)
            | Self::LdiB(_)
            | Self::Cpi(_)
            | Self::Jump(..)
            | Self::Call(_)
            | Self::Djnz(_) => 2,
            _ => 1,
        }
    }

    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Ld(a) => vec![memory(0x1, a)],
            Self::St(a) => vec![memory(0x2, a)],
            Self::Adc(a) => vec![memory(0x4, a)],
            Self::Sbc(a) => vec![memory(0x5, a)],
            Self::Cmp(a) => vec![memory(0x6, a)],
            Self::LdiA(v) => vec![0x30, v],
            Self::LdiB(v) => vec![0x31, v],
            Self::Cpi(v) => vec![0x32, v],
            Self::In(p) => vec![0x70 | (p & 0x0F)],
            Self::Out(p) => vec![0x80 | (p & 0x0F)],
            Self::Jump(c, t) => vec![0xB0 | c.code(), t],
            Self::Call(t) => vec![0xF0, t],
            Self::Djnz(t) => vec![0xF1, t],
            Self::Reserved(b) => vec![b],
            misc => {
                let index = MISC.iter().position(|&m| m == misc).unwrap_or_default();
                vec![index as u8]
            }
        }
    }
}

fn memory(opcode: u8, addr: Addr) -> u8 {
    match addr {
        Addr::Direct(n) => opcode << 4 | (n & 0x0F),
        Addr::Indexed(n) => (opcode | 0x8) << 4 | (n & 0x0F),
    }
}
