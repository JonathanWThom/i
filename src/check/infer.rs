use crate::ast::{Expr, ExprKind, Pattern, PatternKind};
use crate::check::types::{PrimTy, Scheme, Subst, Ty, TyVarId, Typing};
use crate::check::unify::{apply_subst, unify};
use crate::error::Error;
use crate::resolve::{DefId, LocalId, Resolution, ResolvedName};
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PatternResult {
    pub ty: Ty,
    pub bindings: Vec<LocalId>,
}

#[derive(Debug)]
pub struct Infer<'a> {
    pub res: &'a Resolution,
    pub subst: Subst,
    pub locals: HashMap<LocalId, Ty>,
    pub schemes: HashMap<DefId, Scheme>,
    pub errors: Vec<Error>,
    next_var: u32,
    expr_types: HashMap<Span, Ty>,
    pattern_types: HashMap<Span, Ty>,
}

impl<'a> Infer<'a> {
    pub fn new(res: &'a Resolution) -> Self {
        Self {
            res,
            subst: Subst::new(),
            locals: HashMap::new(),
            schemes: HashMap::new(),
            errors: Vec::new(),
            next_var: 0,
            expr_types: HashMap::new(),
            pattern_types: HashMap::new(),
        }
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

    pub fn instantiate(&mut self, scheme: Scheme) -> Ty {
        let mut s: Subst = HashMap::new();
        for v in &scheme.vars {
            let fresh = self.fresh();
            s.insert(*v, Ty::Var(fresh));
        }
        apply_subst(&scheme.ty, &s)
    }

    pub fn infer_expr(&mut self, e: &Expr) -> Ty {
        let ty = match &e.node {
            ExprKind::IntLit(_) => Ty::Prim(PrimTy::Int),
            ExprKind::FloatLit(_) => Ty::Prim(PrimTy::Float),
            ExprKind::StringLit(_) => Ty::Prim(PrimTy::String),
            ExprKind::Var(_) => match self.res.refs.get(&e.span) {
                Some(ResolvedName::TopLevel(def_id)) => match self.schemes.get(def_id).cloned() {
                    Some(scheme) => self.instantiate(scheme),
                    None => Ty::Var(self.fresh()),
                },
                Some(ResolvedName::Local(local_id)) => self
                    .locals
                    .get(local_id)
                    .cloned()
                    .unwrap_or_else(|| Ty::Var(self.fresh())),
                _ => Ty::Var(self.fresh()),
            },
            ExprKind::Ctor(_) => Ty::Var(self.fresh()),
            ExprKind::Lambda { params, body } => {
                let mut param_tys = Vec::with_capacity(params.len());
                for p in params {
                    let pr = self.infer_pattern(p);
                    param_tys.push(pr.ty);
                }
                let result_ty = self.infer_expr(body);
                Ty::Fun(param_tys, Box::new(result_ty))
            }
            ExprKind::Call { func, args } => {
                let fn_ty = self.infer_expr(func);
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.infer_expr(a)).collect();
                let result_v = self.fresh();
                let expected = Ty::Fun(arg_tys, Box::new(Ty::Var(result_v)));
                if let Err(err) = unify(&mut self.subst, &fn_ty, &expected) {
                    self.errors
                        .push(crate::check::unify_error_to_error(func.span, err));
                }
                Ty::Var(result_v)
            }
            ExprKind::Block(items) => {
                use crate::ast::{BlockItem, DeclKind};
                let mut last_ty = Ty::Prim(PrimTy::Unit);
                for item in items {
                    match item {
                        BlockItem::Binding(decl) => {
                            if let DeclKind::Binding {
                                value: Some(value), ..
                            } = &decl.node
                            {
                                let value_ty = self.infer_expr(value);
                                if let Some(ResolvedName::Local(lid)) =
                                    self.res.refs.get(&decl.span)
                                {
                                    self.locals.insert(*lid, value_ty);
                                }
                            }
                            last_ty = Ty::Prim(PrimTy::Unit);
                        }
                        BlockItem::Expr(expr) => {
                            last_ty = self.infer_expr(expr);
                        }
                    }
                }
                last_ty
            }
            _ => Ty::Var(self.fresh()),
        };
        self.record_expr_type(e.span, ty.clone());
        ty
    }

    pub fn infer_pattern(&mut self, p: &Pattern) -> PatternResult {
        let v = self.fresh();
        let ty = Ty::Var(v);
        self.record_pattern_type(p.span, ty.clone());
        match &p.node {
            PatternKind::Var(_) => {
                if let Some(ResolvedName::Local(lid)) = self.res.refs.get(&p.span) {
                    self.locals.insert(*lid, ty.clone());
                    return PatternResult {
                        ty,
                        bindings: vec![*lid],
                    };
                }
                PatternResult {
                    ty,
                    bindings: vec![],
                }
            }
            _ => PatternResult {
                ty,
                bindings: vec![],
            },
        }
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
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let a = infer.fresh();
        let b = infer.fresh();
        assert_ne!(a, b);
    }

    #[test]
    fn record_expr_type_stores_and_applies_subst() {
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let v = infer.fresh();
        infer.subst.insert(v, Ty::Prim(PrimTy::Int));
        let s = Span::new(0, 1);
        infer.record_expr_type(s, Ty::Var(v));
        let typing = infer.into_typing();
        assert_eq!(typing.expr_types.get(&s), Some(&Ty::Prim(PrimTy::Int)));
    }
}
