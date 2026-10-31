use bevy::prelude::*;

/// What a method of the debug port does and which parameters it reads.
///
/// The console reads it for help, completion and to turn its short syntax
/// into the JSON parameters. The method checks its parameters itself.
#[derive(Clone, Debug)]
pub struct DebugMethod {
    name: &'static str,
    summary: &'static str,
    params: Vec<Param>,
}

/// A parameter of a [`DebugMethod`].
#[derive(Clone, Debug)]
pub struct Param {
    pub name: &'static str,
    pub kind: ParamKind,
    pub help: &'static str,
}

/// The JSON value a [`Param`] takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    /// `true` or `false`.
    Flag,
    /// A number, or a list of numbers like a position.
    Numbers,
    /// A string.
    Text,
    /// One of these strings.
    Choice(&'static [&'static str]),
}

/// Every [`DebugMethod`] registered with
/// [`DebugMethods::add_debug_method`](super::DebugMethods::add_debug_method).
#[derive(Resource, Debug, Default)]
pub struct DebugMethodList(Vec<DebugMethod>);

impl DebugMethod {
    pub fn new(name: &'static str, summary: &'static str) -> Self {
        Self {
            name,
            summary,
            params: Vec::new(),
        }
    }

    pub fn param(mut self, name: &'static str, kind: ParamKind, help: &'static str) -> Self {
        self.params.push(Param { name, kind, help });
        self
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn summary(&self) -> &'static str {
        self.summary
    }

    pub fn params(&self) -> &[Param] {
        &self.params
    }
}

impl DebugMethodList {
    pub fn get(&self, name: &str) -> Option<&DebugMethod> {
        self.0.iter().find(|method| method.name == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DebugMethod> {
        self.0.iter()
    }

    pub(super) fn insert(&mut self, method: DebugMethod) {
        self.0.retain(|known| known.name != method.name);
        self.0.push(method);
    }
}

impl FromIterator<DebugMethod> for DebugMethodList {
    fn from_iter<I: IntoIterator<Item = DebugMethod>>(methods: I) -> Self {
        let mut list = Self::default();
        for method in methods {
            list.insert(method);
        }
        list
    }
}
