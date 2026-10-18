use std::collections::BTreeMap;

use crate::Diagnostic;
use crate::ast::BlockKind;
use crate::backend::{Backend, Features};
use crate::hir::{Block, Cond, Node, Place, Prim, Program, Stmt, Value};
use crate::labels::{LabelId, LabelName, Labels};

const MAX_UNROLL: u8 = 16;

type Result<T> = std::result::Result<T, Diagnostic>;

struct Inliner<'a> {
    routines: BTreeMap<LabelId, Block>,
    sites: BTreeMap<LabelId, usize>,
    labels: &'a mut Labels,
}

impl Inliner<'_> {
    fn nodes(&mut self, nodes: Vec<Node>, stack: &mut Vec<LabelId>) -> Result<Vec<Node>> {
        let mut out = Vec::new();
        for node in nodes {
            let Node { stmt, comment, pos } = node;
            let stmt = match stmt {
                Stmt::Prim(Prim::Call(target)) => {
                    let routine = self.routines[&target].clone();
                    if stack.contains(&target) {
                        return Err(Diagnostic::error(
                            pos,
                            format!("{} calls itself", routine.name),
                        ));
                    }
                    stack.push(target);
                    let mut body = self.nodes(routine.body, stack)?;
                    stack.pop();
                    let once = self.sites.get(&target) == Some(&1);
                    if !once {
                        self.relabel(&mut body);
                    }
                    let end = self.labels.add(LabelName::Join(prefix(&routine.name)));
                    if matches!(
                        body.last(),
                        Some(Node {
                            stmt: Stmt::Prim(Prim::Return),
                            ..
                        })
                    ) {
                        body.pop();
                    }
                    let returns = replace_returns(&mut body, end);
                    let mut inner = Vec::new();
                    if once {
                        inner.push(Node {
                            stmt: Stmt::Label(routine.label),
                            comment: None,
                            pos,
                        });
                    }
                    inner.extend(body);
                    if returns {
                        inner.push(Node {
                            stmt: Stmt::Label(end),
                            comment: None,
                            pos,
                        });
                    }
                    Stmt::Inlined {
                        name: routine.name,
                        body: inner,
                    }
                }
                Stmt::If { cond, then, els } => Stmt::If {
                    cond,
                    then: self.nodes(then, stack)?,
                    els: self.nodes(els, stack)?,
                },
                Stmt::Inlined { name, body } => Stmt::Inlined {
                    name,
                    body: self.nodes(body, stack)?,
                },
                Stmt::Repeat { label, body } => Stmt::Repeat {
                    label,
                    body: self.nodes(body, stack)?,
                },
                stmt => stmt,
            };
            out.push(Node { stmt, comment, pos });
        }
        Ok(out)
    }

    /// Gives the labels defined in `body` fresh ids, so a second copy does not clash.
    fn relabel(&mut self, body: &mut [Node]) {
        let mut defined = Vec::new();
        Node::visit(body, &mut |node| match &node.stmt {
            Stmt::Label(id)
            | Stmt::Repeat {
                label: Some(id), ..
            } => defined.push(*id),
            _ => {}
        });
        let fresh: BTreeMap<LabelId, LabelId> = defined
            .into_iter()
            .map(|id| {
                let name = match self.labels.get(id) {
                    LabelName::Fixed(name) | LabelName::Hint(name) => LabelName::Hint(name.clone()),
                    join @ LabelName::Join(_) => join.clone(),
                };
                (id, self.labels.add(name))
            })
            .collect();
        map_labels(body, &|id| fresh.get(&id).copied().unwrap_or(id));
    }
}

/// Makes the program fit the backend: inlines calls and unrolls loops it lacks.
pub(crate) fn legalize(
    mut program: Program,
    backend: &dyn Backend,
) -> std::result::Result<Program, Vec<Diagnostic>> {
    let features = backend.features().clone();
    if !features.calls {
        let mut sites = BTreeMap::new();
        for block in &program.blocks {
            Node::visit(&block.body, &mut |node| {
                if let Stmt::Prim(Prim::Call(target)) = node.stmt {
                    *sites.entry(target).or_insert(0) += 1;
                }
            });
        }
        let (routines, blocks): (Vec<_>, Vec<_>) = program
            .blocks
            .into_iter()
            .partition(|b| b.kind == BlockKind::Routine);
        let mut inliner = Inliner {
            routines: routines.into_iter().map(|r| (r.label, r)).collect(),
            sites,
            labels: &mut program.labels,
        };
        let mut errors = Vec::new();
        let mut inlined = Vec::new();
        for mut block in blocks {
            match inliner.nodes(std::mem::take(&mut block.body), &mut Vec::new()) {
                Ok(body) => block.body = body,
                Err(error) => errors.push(error),
            }
            inlined.push(block);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        program.blocks = inlined;
    }
    let mut errors = Vec::new();
    for block in &mut program.blocks {
        let body = std::mem::take(&mut block.body);
        match unroll(body, &features, backend.name()) {
            Ok(body) => block.body = body,
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(program)
    } else {
        Err(errors)
    }
}

fn map_labels(nodes: &mut [Node], f: &impl Fn(LabelId) -> LabelId) {
    for node in nodes {
        match &mut node.stmt {
            Stmt::Label(id) => *id = f(*id),
            Stmt::Prim(Prim::Jump(id) | Prim::Branch(_, id) | Prim::LoopBack(id)) => {
                *id = f(*id);
            }
            Stmt::If { then, els, .. } => {
                map_labels(then, f);
                map_labels(els, f);
            }
            Stmt::Inlined { body, .. } => map_labels(body, f),
            Stmt::Repeat { label, body } => {
                if let Some(id) = label {
                    *id = f(*id);
                }
                map_labels(body, f);
            }
            _ => {}
        }
    }
}

fn map_places(nodes: &mut [Node], f: &impl Fn(Place) -> Place) {
    let value = |v: &mut Value| {
        if let Value::Place(place) = v {
            *place = f(*place);
        }
    };
    let cond = |c: &mut Cond| {
        if let Cond::Compare(_, v) = c {
            value(v);
        }
    };
    for node in nodes {
        match &mut node.stmt {
            Stmt::Prim(prim) => match prim {
                Prim::AddCarry(v) | Prim::Load(v) | Prim::SubCarry(v) => value(v),
                Prim::Branch(c, _) => cond(c),
                Prim::Move { dst, src } => {
                    *dst = f(*dst);
                    value(src);
                }
                Prim::Store(place) => *place = f(*place),
                _ => {}
            },
            Stmt::If { cond: c, then, els } => {
                cond(c);
                map_places(then, f);
                map_places(els, f);
            }
            Stmt::Inlined { body, .. } | Stmt::Repeat { body, .. } => map_places(body, f),
            Stmt::Asm(_) | Stmt::Comment(_) | Stmt::Label(_) => {}
        }
    }
}

fn prefix(name: &str) -> String {
    name.chars().take(1).collect()
}

/// Turns every `return` into a jump to `end`. Returns whether there was one.
fn replace_returns(nodes: &mut [Node], end: LabelId) -> bool {
    let mut found = false;
    for node in nodes {
        match &mut node.stmt {
            Stmt::Prim(prim @ Prim::Return) => {
                *prim = Prim::Jump(end);
                found = true;
            }
            Stmt::If { then, els, .. } => {
                found |= replace_returns(then, end);
                found |= replace_returns(els, end);
            }
            Stmt::Repeat { body, .. } => found |= replace_returns(body, end),
            _ => {}
        }
    }
    found
}

/// Whether `nodes` may read i.
fn reads_index(nodes: &[Node]) -> bool {
    let mut found = uses_indexed(nodes);
    Node::visit(nodes, &mut |node| {
        found |= matches!(node.stmt, Stmt::Prim(Prim::Call(_)) | Stmt::Asm(_));
    });
    found
}

fn uses_indexed(nodes: &[Node]) -> bool {
    let mut found = false;
    Node::places(nodes, &mut |place| {
        found |= matches!(place, Place::Indexed(_))
    });
    found
}

fn unroll(nodes: Vec<Node>, features: &Features, name: &str) -> Result<Vec<Node>> {
    let mut out: Vec<Node> = Vec::new();
    let mut index: Option<(u8, usize)> = None;
    for node in nodes {
        let Node { stmt, comment, pos } = node;
        match stmt {
            Stmt::Prim(Prim::SetIndex(literal)) => {
                index = Some((literal.value, out.len()));
                out.push(Node {
                    stmt: Stmt::Prim(Prim::SetIndex(literal)),
                    comment,
                    pos,
                });
            }
            Stmt::Repeat { label, body }
                if !features.loops || (!features.indexed && uses_indexed(&body)) =>
            {
                let Some((count, at)) = index.take() else {
                    return Err(Diagnostic::error(
                        pos,
                        format!(
                            "{name} cannot run this loop. Set i to a number right before it so \
                             it can be unrolled"
                        ),
                    ));
                };
                if count == 0 || count > MAX_UNROLL {
                    return Err(Diagnostic::error(
                        pos,
                        format!(
                            "{name} cannot run this loop. Only 1 to {MAX_UNROLL} passes unroll"
                        ),
                    ));
                }
                let mut nested = false;
                Node::visit(&body, &mut |n| {
                    nested |= matches!(
                        n.stmt,
                        Stmt::Label(_) | Stmt::Repeat { .. } | Stmt::Prim(Prim::Call(_))
                    );
                });
                if nested {
                    return Err(Diagnostic::error(
                        pos,
                        "loops with labels, loops or calls inside cannot be unrolled",
                    ));
                }
                let set = out.remove(at);
                if reads_index(&out[at..]) {
                    out.insert(at, set);
                }
                let cells = features.ram_cells;
                for k in (1..=count).rev() {
                    let mut copy = body.clone();
                    map_places(&mut copy, &|place| match place {
                        Place::Indexed(offset) => {
                            let cell = (u16::from(offset) + u16::from(k)) % u16::from(cells);
                            Place::Cell(cell as u8)
                        }
                        place => place,
                    });
                    if k == count {
                        if let Some(label) = label {
                            out.push(Node {
                                stmt: Stmt::Label(label),
                                comment: None,
                                pos,
                            });
                        }
                        if let (Some(first), Some(text)) = (copy.first_mut(), &comment) {
                            first.comment.get_or_insert_with(|| text.clone());
                        }
                    }
                    out.extend(copy);
                }
            }
            Stmt::Repeat { label, body } => {
                index = None;
                out.push(Node {
                    stmt: Stmt::Repeat {
                        label,
                        body: unroll(body, features, name)?,
                    },
                    comment,
                    pos,
                });
            }
            Stmt::If { cond, then, els } => {
                index = None;
                out.push(Node {
                    stmt: Stmt::If {
                        cond,
                        then: unroll(then, features, name)?,
                        els: unroll(els, features, name)?,
                    },
                    comment,
                    pos,
                });
            }
            Stmt::Inlined {
                name: routine,
                body,
            } => {
                index = None;
                out.push(Node {
                    stmt: Stmt::Inlined {
                        name: routine,
                        body: unroll(body, features, name)?,
                    },
                    comment,
                    pos,
                });
            }
            stmt => {
                if matches!(
                    stmt,
                    Stmt::Asm(_) | Stmt::Label(_) | Stmt::Prim(Prim::Call(_))
                ) {
                    index = None;
                }
                out.push(Node { stmt, comment, pos });
            }
        }
    }
    Ok(out)
}
