use crate::check::types::{Subst, Ty, TyVarId};

pub fn apply_subst(ty: &Ty, subst: &Subst) -> Ty {
    match ty {
        Ty::Var(v) => match subst.get(v) {
            Some(t) => apply_subst(t, subst),
            None => Ty::Var(*v),
        },
        Ty::Prim(p) => Ty::Prim(*p),
        Ty::Con(id, args) => Ty::Con(*id, args.iter().map(|a| apply_subst(a, subst)).collect()),
        Ty::Fun(params, result) => Ty::Fun(
            params.iter().map(|p| apply_subst(p, subst)).collect(),
            Box::new(apply_subst(result, subst)),
        ),
    }
}

pub fn occurs(var: TyVarId, ty: &Ty, subst: &Subst) -> bool {
    match apply_subst(ty, subst) {
        Ty::Var(v) => v == var,
        Ty::Prim(_) => false,
        Ty::Con(_, args) => args.iter().any(|a| occurs(var, a, subst)),
        Ty::Fun(ps, r) => ps.iter().any(|p| occurs(var, p, subst)) || occurs(var, &r, subst),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::types::{PrimTy, Ty, TyVarId};
    use std::collections::HashMap;

    #[test]
    fn apply_subst_replaces_bound_var() {
        let mut s: crate::check::types::Subst = HashMap::new();
        s.insert(TyVarId(0), Ty::Prim(PrimTy::Int));
        let ty = Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Var(TyVarId(1))));
        let out = apply_subst(&ty, &s);
        assert_eq!(
            out,
            Ty::Fun(vec![Ty::Prim(PrimTy::Int)], Box::new(Ty::Var(TyVarId(1))))
        );
    }

    #[test]
    fn occurs_finds_var_inside_fun() {
        let ty = Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Int)));
        assert!(occurs(TyVarId(0), &ty, &HashMap::new()));
        assert!(!occurs(TyVarId(1), &ty, &HashMap::new()));
    }
}
