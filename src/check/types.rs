use crate::resolve::DefId;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyVarId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimTy {
    Int,
    Float,
    String,
    Bool,
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Var(TyVarId),
    Prim(PrimTy),
    Con(DefId, Vec<Ty>),
    Fun(Vec<Ty>, Box<Ty>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub vars: Vec<TyVarId>,
    pub ty: Ty,
}

pub type Subst = HashMap<TyVarId, Ty>;

#[derive(Debug, Default, Clone)]
pub struct Typing {
    pub schemes: HashMap<DefId, Scheme>,
    pub expr_types: HashMap<Span, Ty>,
    pub pattern_types: HashMap<Span, Ty>,
}
