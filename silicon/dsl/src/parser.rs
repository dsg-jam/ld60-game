use crate::Diagnostic;
use crate::ast::{
    Arm, AssignOp, BinOp, Block, BlockKind, CellDecl, CmpOp, Cond, ConstDecl, Expr, ExprKind, File,
    FirmwareDecl, Item, Stmt, StmtKind, Switch, SwitchKind,
};
use crate::diagnostic::Pos;
use crate::lexer::{Tok, Token, lex};

const KEYWORDS: &[&str] = &[
    "as", "block", "cell", "cells", "const", "downto", "else", "entry", "firmware", "for", "goto",
    "if", "match", "repeat", "return", "routine", "to", "wait",
];

type Result<T> = std::result::Result<T, Diagnostic>;

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn arm(&mut self) -> Result<Arm> {
        let pos = self.pos();
        let pattern = self.pattern()?;
        self.expect("=>")?;
        self.keyword("goto")?;
        let (target, _) = self.ident()?;
        let comment = self.end()?;
        Ok(Arm {
            pattern,
            target,
            comment,
            pos,
        })
    }

    fn block(&mut self, kind: BlockKind, pos: Pos) -> Result<Block> {
        let (name, _) = self.ident()?;
        let mut params = Vec::new();
        if kind == BlockKind::Routine && self.eat("(") {
            while !self.eat(")") {
                if !params.is_empty() {
                    self.expect(",")?;
                }
                params.push(self.ident()?.0);
            }
        }
        let body = self.body()?;
        Ok(Block {
            kind,
            name,
            params,
            body,
            pos,
        })
    }

    fn body(&mut self) -> Result<Vec<Stmt>> {
        self.expect("{")?;
        let stmts = self.stmts()?;
        self.expect("}")?;
        Ok(stmts)
    }

    fn cond(&mut self) -> Result<Cond> {
        let pos = self.pos();
        if self.eat("!") {
            let (name, _) = self.ident()?;
            return Ok(Cond::Flag {
                name,
                set: false,
                pos,
            });
        }
        let lhs = self.expr()?;
        let op = match self.peek() {
            Tok::Sym("==") => CmpOp::Eq,
            Tok::Sym("!=") => CmpOp::Ne,
            Tok::Sym("<") => CmpOp::Lt,
            Tok::Sym(">=") => CmpOp::Ge,
            _ => {
                return match lhs.kind {
                    ExprKind::Name(name) => Ok(Cond::Flag {
                        name,
                        set: true,
                        pos,
                    }),
                    _ => Err(self.unexpected("a comparison")),
                };
            }
        };
        self.at += 1;
        let rhs = self.expr()?;
        Ok(Cond::Compare { lhs, op, rhs })
    }

    fn eat(&mut self, sym: &str) -> bool {
        let found = matches!(self.peek(), Tok::Sym(s) if *s == sym);
        if found {
            self.at += 1;
        }
        found
    }

    fn eat_keyword(&mut self, word: &str) -> bool {
        let found = matches!(self.peek(), Tok::Ident(s) if s == word);
        if found {
            self.at += 1;
        }
        found
    }

    /// Ends a statement: an optional `;` comment, then a newline or `}`.
    fn end(&mut self) -> Result<Option<String>> {
        let comment = match self.peek() {
            Tok::Comment(text) => {
                let text = text.clone();
                self.at += 1;
                Some(text)
            }
            _ => None,
        };
        match self.peek() {
            Tok::Newline => self.at += 1,
            Tok::Eof | Tok::Sym("}") => {}
            _ => return Err(self.unexpected("the end of the line")),
        }
        Ok(comment)
    }

    fn expect(&mut self, sym: &str) -> Result<()> {
        if self.eat(sym) {
            Ok(())
        } else {
            Err(self.unexpected(&format!("'{sym}'")))
        }
    }

    fn expr(&mut self) -> Result<Expr> {
        let mut lhs = self.primary()?;
        loop {
            let op = match self.peek() {
                Tok::Sym("+") => BinOp::Add,
                Tok::Sym("-") => BinOp::Sub,
                _ => return Ok(lhs),
            };
            self.at += 1;
            let rhs = self.primary()?;
            let pos = lhs.pos;
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                pos,
            };
        }
    }

    fn ident(&mut self) -> Result<(String, Pos)> {
        let pos = self.pos();
        match self.peek() {
            Tok::Ident(name) => {
                let name = name.clone();
                self.at += 1;
                Ok((name, pos))
            }
            _ => Err(self.unexpected("a name")),
        }
    }

    fn item(&mut self) -> Result<Item> {
        let pos = self.pos();
        if let Tok::Comment(text) = self.peek() {
            let text = text.clone();
            self.at += 1;
            self.end()?;
            return Ok(Item::Comment(text));
        }
        let (word, _) = self.ident()?;
        let item = match word.as_str() {
            "firmware" => {
                let title = self.string()?;
                self.keyword("for")?;
                let (device, _) = self.ident()?;
                Item::Firmware(FirmwareDecl { title, device, pos })
            }
            "cell" | "cells" => {
                let (name, _) = self.ident()?;
                let len = if word == "cells" {
                    self.expect("[")?;
                    let len = self.number()?;
                    self.expect("]")?;
                    Some(len)
                } else {
                    None
                };
                self.keyword("at")?;
                let (addr, _) = self.ident()?;
                let comment = self.end()?;
                return Ok(Item::Cell(CellDecl {
                    name,
                    len,
                    addr,
                    comment,
                    pos,
                }));
            }
            "const" => {
                let (name, _) = self.ident()?;
                self.expect("=")?;
                let value = match self.peek() {
                    Tok::Number(literal) => literal.clone(),
                    _ => return Err(self.unexpected("a number")),
                };
                self.at += 1;
                Item::Const(ConstDecl { name, value, pos })
            }
            "entry" => Item::Block(self.block(BlockKind::Entry, pos)?),
            "block" => Item::Block(self.block(BlockKind::Block, pos)?),
            "routine" => Item::Block(self.block(BlockKind::Routine, pos)?),
            word => match SwitchKind::from_keyword(word) {
                Some(kind) => Item::Switch(self.switch(kind, pos, Self::items)?),
                None => {
                    return Err(Diagnostic::error(
                        pos,
                        format!("expected a declaration, found '{word}'"),
                    ));
                }
            },
        };
        self.end()?;
        Ok(item)
    }

    fn items(&mut self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Eof | Tok::Sym("}")) {
                return Ok(items);
            }
            items.push(self.item()?);
        }
    }

    fn keyword(&mut self, word: &str) -> Result<()> {
        if self.eat_keyword(word) {
            Ok(())
        } else {
            Err(self.unexpected(&format!("'{word}'")))
        }
    }

    fn number(&mut self) -> Result<u8> {
        match self.peek() {
            Tok::Number(literal) => {
                let value = literal.value;
                self.at += 1;
                Ok(value)
            }
            _ => Err(self.unexpected("a number")),
        }
    }

    fn pattern(&mut self) -> Result<Expr> {
        let lo = self.expr()?;
        if !self.eat_keyword("to") {
            return Ok(lo);
        }
        let hi = self.expr()?;
        let pos = lo.pos;
        Ok(Expr {
            kind: ExprKind::Range(Box::new(lo), Box::new(hi)),
            pos,
        })
    }

    fn peek(&self) -> &Tok {
        &self.tokens[self.at].tok
    }

    fn pos(&self) -> Pos {
        self.tokens[self.at].pos
    }

    fn primary(&mut self) -> Result<Expr> {
        let pos = self.pos();
        let kind = match self.peek().clone() {
            Tok::Number(literal) => {
                self.at += 1;
                ExprKind::Num(literal)
            }
            Tok::Key(key) => {
                self.at += 1;
                ExprKind::Key(key)
            }
            Tok::Ident(name) if !KEYWORDS.contains(&name.as_str()) => {
                self.at += 1;
                if let Some(kind) = SwitchKind::from_keyword(&name) {
                    let switch = self.switch(kind, pos, |p| {
                        p.skip_newlines();
                        let expr = p.pattern()?;
                        p.skip_newlines();
                        Ok(vec![expr])
                    })?;
                    ExprKind::Switch(Box::new(switch))
                } else if self.eat("[") {
                    let index = self.expr()?;
                    self.expect("]")?;
                    ExprKind::Index(name, Box::new(index))
                } else if self.eat("(") {
                    ExprKind::Call(name, self.args()?)
                } else {
                    ExprKind::Name(name)
                }
            }
            _ => return Err(self.unexpected("a value")),
        };
        Ok(Expr { kind, pos })
    }

    fn args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        while !self.eat(")") {
            if !args.is_empty() {
                self.expect(",")?;
            }
            args.push(self.expr()?);
        }
        Ok(args)
    }

    fn skip_newlines(&mut self) {
        while *self.peek() == Tok::Newline {
            self.at += 1;
        }
    }

    fn stmt(&mut self) -> Result<Stmt> {
        let pos = self.pos();
        let kind = match self.peek().clone() {
            Tok::Comment(text) => {
                self.at += 1;
                self.end()?;
                return Ok(Stmt {
                    kind: StmtKind::Comment(text),
                    comment: None,
                    pos,
                });
            }
            Tok::Asm(lines) => {
                self.at += 1;
                StmtKind::Asm(lines)
            }
            Tok::Ident(word) => self.stmt_kind(&word, pos)?,
            _ => return Err(self.unexpected("a statement")),
        };
        let comment = self.end()?;
        Ok(Stmt { kind, comment, pos })
    }

    fn stmt_kind(&mut self, word: &str, pos: Pos) -> Result<StmtKind> {
        if let Some(kind) = SwitchKind::from_keyword(word) {
            self.at += 1;
            return Ok(StmtKind::Switch(self.switch(kind, pos, Self::stmts)?));
        }
        let keyword = match word {
            "wait" | "return" | "goto" | "if" | "repeat" | "for" | "match" => {
                self.at += 1;
                true
            }
            _ => false,
        };
        if keyword {
            return self.keyword_stmt(word);
        }
        let (name, _) = self.ident()?;
        if self.eat(":") {
            return Ok(StmtKind::Label(name));
        }
        if self.eat("(") {
            let args = self.args()?;
            return Ok(StmtKind::Call { name, args });
        }
        self.at -= 1;
        let target = self.primary()?;
        let op = if self.eat("=") {
            AssignOp::Set
        } else if self.eat("+=") {
            AssignOp::Add
        } else if self.eat("-=") {
            AssignOp::Sub
        } else {
            return Err(self.unexpected("'=', '+=' or '-='"));
        };
        let value = self.expr()?;
        Ok(StmtKind::Assign { target, op, value })
    }

    fn keyword_stmt(&mut self, word: &str) -> Result<StmtKind> {
        Ok(match word {
            "wait" => StmtKind::Wait,
            "return" => StmtKind::Return,
            "goto" => StmtKind::Goto(self.ident()?.0),
            "if" => {
                let cond = self.cond()?;
                if self.eat_keyword("goto") {
                    let (target, _) = self.ident()?;
                    return Ok(StmtKind::IfGoto { cond, target });
                }
                let then = self.body()?;
                let els = if self.eat_keyword("else") {
                    self.body()?
                } else {
                    Vec::new()
                };
                StmtKind::If { cond, then, els }
            }
            "repeat" => {
                let (counter, _) = self.ident()?;
                let label = self.loop_label()?;
                let body = self.body()?;
                StmtKind::Repeat {
                    counter,
                    label,
                    body,
                }
            }
            "for" => {
                let (counter, _) = self.ident()?;
                self.expect("=")?;
                let start = self.expr()?;
                self.keyword("downto")?;
                let end = self.expr()?;
                let label = self.loop_label()?;
                let body = self.body()?;
                StmtKind::For {
                    counter,
                    start,
                    end,
                    label,
                    body,
                }
            }
            _ => {
                let scrutinee = self.expr()?;
                self.expect("{")?;
                let mut arms = Vec::new();
                loop {
                    self.skip_newlines();
                    if self.eat("}") {
                        break;
                    }
                    arms.push(self.arm()?);
                }
                StmtKind::Match { scrutinee, arms }
            }
        })
    }

    fn loop_label(&mut self) -> Result<Option<String>> {
        if self.eat_keyword("as") {
            Ok(Some(self.ident()?.0))
        } else {
            Ok(None)
        }
    }

    fn stmts(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Eof | Tok::Sym("}")) {
                return Ok(stmts);
            }
            stmts.push(self.stmt()?);
        }
    }

    fn string(&mut self) -> Result<String> {
        match self.peek() {
            Tok::Str(text) => {
                let text = text.clone();
                self.at += 1;
                Ok(text)
            }
            _ => Err(self.unexpected("a string")),
        }
    }

    fn switch<T>(
        &mut self,
        kind: SwitchKind,
        pos: Pos,
        mut side: impl FnMut(&mut Self) -> Result<Vec<T>>,
    ) -> Result<Switch<T>> {
        let name = self.string()?;
        self.expect("{")?;
        let [default_name, other_name] = kind.sides();
        let mut default = None;
        let mut other = None;
        loop {
            self.skip_newlines();
            if self.eat("}") {
                break;
            }
            let (word, word_pos) = self.ident()?;
            let slot = if word == default_name {
                &mut default
            } else if word == other_name {
                &mut other
            } else {
                return Err(Diagnostic::error(
                    word_pos,
                    format!("expected '{default_name}' or '{other_name}', found '{word}'"),
                ));
            };
            if slot.is_some() {
                return Err(Diagnostic::error(
                    word_pos,
                    format!("'{word}' appears twice"),
                ));
            }
            self.expect("{")?;
            *slot = Some(side(self)?);
            self.expect("}")?;
        }
        Ok(Switch {
            kind,
            name,
            default: default.unwrap_or_default(),
            other: other.unwrap_or_default(),
            pos,
        })
    }

    fn unexpected(&self, expected: &str) -> Diagnostic {
        let found = match self.peek() {
            Tok::Asm(_) => "an asm block".into(),
            Tok::Comment(_) => "a comment".into(),
            Tok::Eof => "the end of the file".into(),
            Tok::Ident(name) => format!("'{name}'"),
            Tok::Key(key) => format!("'{key}'"),
            Tok::Newline => "the end of the line".into(),
            Tok::Number(literal) => format!("'{}'", literal.text),
            Tok::Str(text) => format!("\"{text}\""),
            Tok::Sym(sym) => format!("'{sym}'"),
        };
        Diagnostic::error(self.pos(), format!("expected {expected}, found {found}"))
    }
}

pub(crate) fn parse(source: &str) -> Result<File> {
    let mut parser = Parser {
        tokens: lex(source)?,
        at: 0,
    };
    let items = parser.items()?;
    if *parser.peek() != Tok::Eof {
        return Err(parser.unexpected("a declaration"));
    }
    Ok(File { items })
}
