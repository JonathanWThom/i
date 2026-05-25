use crate::check::types::{Scheme, Ty, TyVarId};
use crate::resolve::DefId;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub scheme: Scheme,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub enum PayloadShape {
    Bare,
    Single(Ty),
    Record(Vec<FieldInfo>),
}

#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub ctor_def_id: DefId,
    pub payload: PayloadShape,
    pub parent: DefId,
}

#[derive(Debug, Clone)]
pub enum TypeDeclBody {
    Newtype(Ty),
    Record(Vec<FieldInfo>),
    Sum(Vec<VariantInfo>),
}

#[derive(Debug, Clone)]
pub struct TypeDeclInfo {
    pub def_id: DefId,
    pub name: String,
    pub params: Vec<TyVarId>,
    pub body: TypeDeclBody,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Default, Clone)]
pub struct TypeRegistry {
    pub types: HashMap<DefId, TypeDeclInfo>,
    pub ctor_to_type: HashMap<DefId, DefId>,
}
