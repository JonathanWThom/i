use crate::check::types::{EffectRow, Subst, Ty, TyVarId};

pub fn apply_subst(ty: &Ty, subst: &Subst) -> Ty {
    match ty {
        Ty::Var(v) => match subst.tys.get(v) {
            Some(t) => apply_subst(t, subst),
            None => Ty::Var(*v),
        },
        Ty::Prim(p) => Ty::Prim(*p),
        Ty::Con(id, args) => Ty::Con(*id, args.iter().map(|a| apply_subst(a, subst)).collect()),
        Ty::Fun(params, row, result) => Ty::Fun(
            params.iter().map(|p| apply_subst(p, subst)).collect(),
            apply_eff_row(row, subst),
            Box::new(apply_subst(result, subst)),
        ),
    }
}

/// Resolve an effect row's tail through the substitution, folding any resolved
/// labels into the row. Returns a row whose tail is either None or an unbound
/// effect var.
pub fn apply_eff_row(row: &EffectRow, subst: &Subst) -> EffectRow {
    let mut labels = row.labels;
    let mut tail = row.tail;
    // No visited-guard: termination relies on `effs` being acyclic, which the
    // eff occurs-check in `bind_eff` guarantees (Task 3). Nothing writes `effs`
    // before then, so this can't spin today either.
    while let Some(v) = tail {
        match subst.effs.get(&v) {
            Some(next) => {
                labels = labels.union(next.labels);
                tail = next.tail;
            }
            None => break,
        }
    }
    EffectRow { labels, tail }
}

pub fn occurs(var: TyVarId, ty: &Ty, subst: &Subst) -> bool {
    match apply_subst(ty, subst) {
        Ty::Var(v) => v == var,
        Ty::Prim(_) => false,
        Ty::Con(_, args) => args.iter().any(|a| occurs(var, a, subst)),
        Ty::Fun(ps, _row, r) => ps.iter().any(|p| occurs(var, p, subst)) || occurs(var, &r, subst),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnifyError {
    Mismatch { left: Ty, right: Ty },
    Occurs(TyVarId),
    Arity { expected: usize, found: usize },
}

pub fn unify(subst: &mut Subst, a: &Ty, b: &Ty) -> Result<(), UnifyError> {
    let a = apply_subst(a, subst);
    let b = apply_subst(b, subst);
    match (a, b) {
        (Ty::Var(x), Ty::Var(y)) if x == y => Ok(()),
        (Ty::Var(v), t) | (t, Ty::Var(v)) => {
            if occurs(v, &t, subst) {
                Err(UnifyError::Occurs(v))
            } else {
                subst.tys.insert(v, t);
                Ok(())
            }
        }
        (Ty::Prim(p), Ty::Prim(q)) if p == q => Ok(()),
        (Ty::Con(id1, args1), Ty::Con(id2, args2)) if id1 == id2 => {
            if args1.len() != args2.len() {
                return Err(UnifyError::Arity {
                    expected: args1.len(),
                    found: args2.len(),
                });
            }
            for (x, y) in args1.iter().zip(args2.iter()) {
                unify(subst, x, y)?;
            }
            Ok(())
        }
        (Ty::Fun(p1, _e1, r1), Ty::Fun(p2, _e2, r2)) => {
            if p1.len() != p2.len() {
                return Err(UnifyError::Arity {
                    expected: p1.len(),
                    found: p2.len(),
                });
            }
            for (x, y) in p1.iter().zip(p2.iter()) {
                unify(subst, x, y)?;
            }
            unify(subst, &r1, &r2)
        }
        (left, right) => Err(UnifyError::Mismatch { left, right }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::types::{EffectRow, PrimTy, Subst, Ty, TyVarId};

    #[test]
    fn apply_subst_replaces_bound_var() {
        let mut s = Subst::new();
        s.tys.insert(TyVarId(0), Ty::Prim(PrimTy::Int));
        let ty = Ty::Fun(
            vec![Ty::Var(TyVarId(0))],
            EffectRow::pure(),
            Box::new(Ty::Var(TyVarId(1))),
        );
        let out = apply_subst(&ty, &s);
        assert_eq!(
            out,
            Ty::Fun(
                vec![Ty::Prim(PrimTy::Int)],
                EffectRow::pure(),
                Box::new(Ty::Var(TyVarId(1)))
            )
        );
    }

    #[test]
    fn occurs_finds_var_inside_fun() {
        let ty = Ty::Fun(
            vec![Ty::Var(TyVarId(0))],
            EffectRow::pure(),
            Box::new(Ty::Prim(PrimTy::Int)),
        );
        assert!(occurs(TyVarId(0), &ty, &Subst::new()));
        assert!(!occurs(TyVarId(1), &ty, &Subst::new()));
    }

    #[test]
    fn unify_primitive_succeeds_when_equal() {
        let mut s = Subst::new();
        unify(&mut s, &Ty::Prim(PrimTy::Int), &Ty::Prim(PrimTy::Int)).unwrap();
        assert!(s.tys.is_empty());
    }

    #[test]
    fn unify_primitive_fails_when_distinct() {
        let mut s = Subst::new();
        let r = unify(&mut s, &Ty::Prim(PrimTy::Int), &Ty::Prim(PrimTy::Float));
        assert!(matches!(r, Err(UnifyError::Mismatch { .. })));
    }

    #[test]
    fn unify_var_binds_when_unbound() {
        let mut s = Subst::new();
        unify(&mut s, &Ty::Var(TyVarId(0)), &Ty::Prim(PrimTy::Int)).unwrap();
        assert_eq!(s.tys.get(&TyVarId(0)), Some(&Ty::Prim(PrimTy::Int)));
    }

    #[test]
    fn unify_var_with_self_containing_term_is_occurs_check() {
        let mut s = Subst::new();
        let lhs = Ty::Var(TyVarId(0));
        let rhs = Ty::Fun(
            vec![Ty::Var(TyVarId(0))],
            EffectRow::pure(),
            Box::new(Ty::Prim(PrimTy::Int)),
        );
        let r = unify(&mut s, &lhs, &rhs);
        assert!(matches!(r, Err(UnifyError::Occurs(_))));
    }

    #[test]
    fn apply_subst_resolves_effect_tail_in_fun_row() {
        use crate::check::types::{Effect, EffectSet, EffectVarId};
        let mut s = Subst::default();
        s.effs.insert(
            EffectVarId(0),
            EffectRow::concrete(EffectSet::single(Effect::Io)),
        );
        let f = Ty::Fun(
            vec![],
            EffectRow::open(EffectSet::empty(), EffectVarId(0)),
            Box::new(Ty::Prim(PrimTy::Unit)),
        );
        let out = apply_subst(&f, &s);
        match out {
            Ty::Fun(_, row, _) => {
                assert!(row.labels.contains(Effect::Io));
                assert!(row.tail.is_none());
            }
            _ => panic!("expected Fun"),
        }
    }

    #[test]
    fn unify_fun_compares_arities() {
        let mut s = Subst::new();
        let one = Ty::Fun(
            vec![Ty::Prim(PrimTy::Int)],
            EffectRow::pure(),
            Box::new(Ty::Prim(PrimTy::Int)),
        );
        let two = Ty::Fun(
            vec![Ty::Prim(PrimTy::Int), Ty::Prim(PrimTy::Int)],
            EffectRow::pure(),
            Box::new(Ty::Prim(PrimTy::Int)),
        );
        assert!(matches!(
            unify(&mut s, &one, &two),
            Err(UnifyError::Arity { .. })
        ));
    }
}
