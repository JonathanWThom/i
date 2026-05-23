use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

pub type ModulePath = Vec<String>;

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedName {
    Local(LocalId),
    TopLevel(DefId),
    Ctor(DefId),
    Imported { module: ModulePath, name: String },
    Module(ModulePath),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefKind {
    Value,
    Type,
    Ctor { of_type: DefId },
    Trait,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefInfo {
    pub id: DefId,
    pub name: String,
    pub kind: DefKind,
    pub span: Span,
}

#[derive(Debug, Default, Clone)]
pub struct Resolution {
    pub defs: Vec<DefInfo>,
    pub refs: HashMap<Span, ResolvedName>,
}
