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

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "defs:")?;
        for d in &self.defs {
            writeln!(f, "  {:?} {} ({:?})", d.id, d.name, d.kind)?;
        }
        writeln!(f, "refs:")?;
        let mut entries: Vec<_> = self.refs.iter().collect();
        entries.sort_by_key(|(s, _)| (s.start, s.end));
        for (span, name) in entries {
            writeln!(f, "  {:?} -> {:?}", span, name)?;
        }
        Ok(())
    }
}
