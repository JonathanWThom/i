use crate::check::types::{Scheme, Subst, Ty, TyVarId, Typing};
use crate::check::unify::apply_subst;
use crate::error::Error;
use crate::resolve::{DefId, LocalId};
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Infer {
    pub subst: Subst,
    pub locals: HashMap<LocalId, Ty>,
    pub schemes: HashMap<DefId, Scheme>,
    pub errors: Vec<Error>,
    next_var: u32,
    expr_types: HashMap<Span, Ty>,
    pattern_types: HashMap<Span, Ty>,
}

impl Infer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fresh(&mut self) -> TyVarId {
        let id = TyVarId(self.next_var);
        self.next_var += 1;
        id
    }

    pub fn record_expr_type(&mut self, span: Span, ty: Ty) {
        self.expr_types.insert(span, ty);
    }

    pub fn record_pattern_type(&mut self, span: Span, ty: Ty) {
        self.pattern_types.insert(span, ty);
    }

    pub fn into_typing(self) -> Typing {
        let Infer {
            subst,
            schemes,
            expr_types,
            pattern_types,
            ..
        } = self;
        let expr_types = expr_types
            .into_iter()
            .map(|(s, t)| (s, apply_subst(&t, &subst)))
            .collect();
        let pattern_types = pattern_types
            .into_iter()
            .map(|(s, t)| (s, apply_subst(&t, &subst)))
            .collect();
        Typing {
            schemes,
            expr_types,
            pattern_types,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_returns_distinct_ids() {
        let mut infer = Infer::new();
        let a = infer.fresh();
        let b = infer.fresh();
        assert_ne!(a, b);
    }

    #[test]
    fn record_expr_type_stores_and_applies_subst() {
        use crate::check::types::{PrimTy, Ty, TyVarId};
        use crate::span::Span;

        let mut infer = Infer::new();
        let v = infer.fresh();
        infer.subst.insert(v, Ty::Prim(PrimTy::Int));
        let s = Span::new(0, 1);
        infer.record_expr_type(s, Ty::Var(v));
        let typing = infer.into_typing();
        assert_eq!(typing.expr_types.get(&s), Some(&Ty::Prim(PrimTy::Int)));
        // silence unused import on TyVarId
        let _ = TyVarId(0);
    }
}
