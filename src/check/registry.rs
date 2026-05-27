use crate::check::traits::TraitId;
use crate::check::types::{PrimTy, Scheme, Ty, TyVarId};
use crate::resolve::DefId;
use std::collections::HashMap;

/// The "head" of a type — what an impl matches on. Primitives carry no DefId,
/// so the head unifies the primitive and nominal cases under one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeHead {
    Prim(PrimTy),
    Con(DefId),
}

#[derive(Debug, Clone)]
pub struct ImplInfo {
    pub trait_: TraitId,
    pub head: TypeHead,
}

/// The matchable head of a resolved type. A type variable or function type has
/// no head — neither can carry an impl in Plan 5.
pub fn head_of(ty: &Ty) -> Option<TypeHead> {
    match ty {
        Ty::Prim(p) => Some(TypeHead::Prim(*p)),
        Ty::Con(id, _) => Some(TypeHead::Con(*id)),
        Ty::Var(_) | Ty::Fun(..) => None,
    }
}

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
    pub impls: HashMap<(TraitId, TypeHead), ImplInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_of_classifies_types() {
        assert_eq!(
            head_of(&Ty::Prim(PrimTy::Int)),
            Some(TypeHead::Prim(PrimTy::Int))
        );
        assert_eq!(
            head_of(&Ty::Con(DefId(3), vec![])),
            Some(TypeHead::Con(DefId(3)))
        );
        assert_eq!(head_of(&Ty::Var(TyVarId(0))), None);
    }

    #[test]
    fn impl_table_keys_on_trait_and_head() {
        let mut reg = TypeRegistry::default();
        reg.impls.insert(
            (TraitId::Eq, TypeHead::Con(DefId(3))),
            ImplInfo {
                trait_: TraitId::Eq,
                head: TypeHead::Con(DefId(3)),
            },
        );
        assert!(
            reg.impls
                .contains_key(&(TraitId::Eq, TypeHead::Con(DefId(3))))
        );
        assert!(
            !reg.impls
                .contains_key(&(TraitId::Eq, TypeHead::Prim(PrimTy::Int)))
        );
    }
}
