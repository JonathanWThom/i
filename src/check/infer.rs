use crate::ast::{BinOp, Expr, ExprKind, LitPat, Pattern, PatternKind, UnaryOp};
use crate::check::registry::TypeRegistry;
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
    pub registry: TypeRegistry,
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
            registry: TypeRegistry::default(),
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

    pub fn lower_type(&mut self, t: &crate::ast::Type) -> Ty {
        self.lower_type_in_scope(t, &HashMap::new())
    }

    pub fn lower_type_in_scope(
        &mut self,
        t: &crate::ast::Type,
        ctx: &HashMap<String, TyVarId>,
    ) -> Ty {
        use crate::ast::{EffectRow, TypeKind};
        match &t.node {
            TypeKind::Var(name) => match ctx.get(name) {
                Some(v) => Ty::Var(*v),
                None => Ty::Var(self.fresh()),
            },
            TypeKind::Named { name, args } => match name.as_str() {
                "Int" => Ty::Prim(PrimTy::Int),
                "Float" => Ty::Prim(PrimTy::Float),
                "String" => Ty::Prim(PrimTy::String),
                "Bool" => Ty::Prim(PrimTy::Bool),
                "Unit" => Ty::Prim(PrimTy::Unit),
                _ => match self.res.defs.iter().find(|d| &d.name == name) {
                    Some(d) => {
                        let id = d.id;
                        Ty::Con(
                            id,
                            args.iter()
                                .map(|a| self.lower_type_in_scope(a, ctx))
                                .collect(),
                        )
                    }
                    None => {
                        self.errors.push(Error {
                            span: t.span,
                            kind: crate::error::ErrorKind::Unresolved { name: name.clone() },
                        });
                        Ty::Var(self.fresh())
                    }
                },
            },
            TypeKind::Function {
                params,
                effect,
                result,
            } => {
                if matches!(effect, Some(EffectRow::Named(_))) {
                    self.errors.push(Error {
                        span: t.span,
                        kind: crate::error::ErrorKind::EffectsNotYetImplemented,
                    });
                }
                let ps: Vec<Ty> = params
                    .iter()
                    .map(|p| self.lower_type_in_scope(p, ctx))
                    .collect();
                let r = self.lower_type_in_scope(result, ctx);
                Ty::Fun(ps, Box::new(r))
            }
            TypeKind::Tuple(_) => {
                self.errors.push(Error {
                    span: t.span,
                    kind: crate::error::ErrorKind::TuplesNotYetImplemented,
                });
                Ty::Var(self.fresh())
            }
        }
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
            ExprKind::Ctor(name) => match self.res.refs.get(&e.span).cloned() {
                Some(ResolvedName::Ctor(ctor_id)) => match self.schemes.get(&ctor_id).cloned() {
                    Some(scheme) => self.instantiate(scheme),
                    None => Ty::Var(self.fresh()),
                },
                _ => {
                    self.errors.push(Error {
                        span: e.span,
                        kind: crate::error::ErrorKind::Unresolved { name: name.clone() },
                    });
                    Ty::Var(self.fresh())
                }
            },
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
            ExprKind::Paren(inner) => self.infer_expr(inner),
            ExprKind::Construct { type_name, fields } => self.infer_construct(e, type_name, fields),
            ExprKind::Update { value, fields } => self.infer_update(e, value, fields),
            ExprKind::FieldAccess { receiver, field } => {
                self.infer_field_access(e, receiver, field)
            }
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee_ty = self.infer_expr(scrutinee);
                let result_v = self.fresh();
                for arm in arms {
                    let pat = self.infer_pattern(&arm.pattern);
                    if let Err(ue) = unify(&mut self.subst, &scrutinee_ty, &pat.ty) {
                        self.errors
                            .push(crate::check::unify_error_to_error(arm.pattern.span, ue));
                    }
                    let body_ty = self.infer_expr(&arm.body);
                    if let Err(ue) = unify(&mut self.subst, &Ty::Var(result_v), &body_ty) {
                        self.errors
                            .push(crate::check::unify_error_to_error(arm.body.span, ue));
                    }
                }
                let scrutinee_resolved = apply_subst(&scrutinee_ty, &self.subst);
                if let crate::check::exhaust::Coverage::Missing(missing) =
                    crate::check::exhaust::check_arms(&scrutinee_resolved, arms, &self.registry)
                {
                    self.errors.push(Error {
                        span: e.span,
                        kind: crate::error::ErrorKind::NonExhaustiveMatch { missing },
                    });
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
            ExprKind::BinOp { op, lhs, rhs } => self.infer_binop(e, op, lhs, rhs),
            ExprKind::UnaryOp { op, expr } => self.infer_unaryop(e, op, expr),
            ExprKind::List(items) => self.infer_list(e, items),
            _ => Ty::Var(self.fresh()),
        };
        self.record_expr_type(e.span, ty.clone());
        ty
    }

    fn infer_binop(&mut self, e: &Expr, op: &BinOp, lhs: &Expr, rhs: &Expr) -> Ty {
        let lhs_ty = self.infer_expr(lhs);
        let rhs_ty = self.infer_expr(rhs);
        // Operands must agree, so unify the two sides up front for every
        // operator. What differs per operator is which concrete type the
        // result is and whether the agreed-on operand type is constrained.
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
                self.unify_operands(&lhs_ty, &rhs_ty, e.span);
                let resolved = apply_subst(&lhs_ty, &self.subst);
                self.require_numeric(&resolved, e.span);
                resolved
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.unify_operands(&lhs_ty, &rhs_ty, e.span);
                let resolved = apply_subst(&lhs_ty, &self.subst);
                self.require_numeric(&resolved, e.span);
                Ty::Prim(PrimTy::Bool)
            }
            BinOp::Eq | BinOp::Ne => {
                self.unify_operands(&lhs_ty, &rhs_ty, e.span);
                Ty::Prim(PrimTy::Bool)
            }
            BinOp::And | BinOp::Or | BinOp::Xor => {
                self.expect_operand(&lhs_ty, &Ty::Prim(PrimTy::Bool), lhs.span);
                self.expect_operand(&rhs_ty, &Ty::Prim(PrimTy::Bool), rhs.span);
                Ty::Prim(PrimTy::Bool)
            }
            BinOp::Concat => {
                self.expect_operand(&lhs_ty, &Ty::Prim(PrimTy::String), lhs.span);
                self.expect_operand(&rhs_ty, &Ty::Prim(PrimTy::String), rhs.span);
                Ty::Prim(PrimTy::String)
            }
        }
    }

    fn infer_unaryop(&mut self, e: &Expr, op: &UnaryOp, inner: &Expr) -> Ty {
        let inner_ty = self.infer_expr(inner);
        match op {
            UnaryOp::Neg => {
                let resolved = apply_subst(&inner_ty, &self.subst);
                if self.require_numeric(&resolved, e.span) {
                    resolved
                } else {
                    Ty::Var(self.fresh())
                }
            }
            UnaryOp::Not => {
                self.expect_operand(&inner_ty, &Ty::Prim(PrimTy::Bool), e.span);
                Ty::Prim(PrimTy::Bool)
            }
        }
    }

    fn infer_list(&mut self, e: &Expr, items: &[Expr]) -> Ty {
        // A list literal's type is `List elem`. `List` is an ordinary library
        // type, not a builtin, so it must be in scope; absent it, there's no
        // type to give the literal.
        let Some(list_id) = self
            .res
            .defs
            .iter()
            .find(|d| d.name == "List")
            .map(|d| d.id)
        else {
            self.errors.push(Error {
                span: e.span,
                kind: crate::error::ErrorKind::UnknownType {
                    name: "List".into(),
                },
            });
            return Ty::Var(self.fresh());
        };
        let elem = Ty::Var(self.fresh());
        for item in items {
            let it = self.infer_expr(item);
            if let Err(ue) = unify(&mut self.subst, &elem, &it) {
                self.errors
                    .push(crate::check::unify_error_to_error(item.span, ue));
            }
        }
        Ty::Con(list_id, vec![elem])
    }

    fn unify_operands(&mut self, lhs: &Ty, rhs: &Ty, span: Span) {
        if let Err(ue) = unify(&mut self.subst, lhs, rhs) {
            self.errors
                .push(crate::check::unify_error_to_error(span, ue));
        }
    }

    fn expect_operand(&mut self, actual: &Ty, expected: &Ty, span: Span) {
        if let Err(ue) = unify(&mut self.subst, actual, expected) {
            self.errors
                .push(crate::check::unify_error_to_error(span, ue));
        }
    }

    /// Arithmetic, ordering and negation accept only Int or Float. There is no
    /// type-variable that captures "numeric", so we check the resolved type
    /// directly instead of unifying. Returns whether the type was acceptable.
    fn require_numeric(&mut self, ty: &Ty, span: Span) -> bool {
        match ty {
            Ty::Prim(PrimTy::Int) | Ty::Prim(PrimTy::Float) => true,
            _ => {
                self.errors.push(Error {
                    span,
                    kind: crate::error::ErrorKind::TypeMismatch {
                        expected: "Int or Float".into(),
                        found: format!("{ty}"),
                    },
                });
                false
            }
        }
    }

    fn infer_construct(&mut self, e: &Expr, type_name: &str, fields: &[crate::ast::KwArg]) -> Ty {
        let def_id = match self.res.refs.get(&e.span) {
            Some(ResolvedName::TopLevel(id)) => *id,
            // Ctor-result construction (sum variants) is Task 16's job.
            _ => return Ty::Var(self.fresh()),
        };
        let info = match self.registry.types.get(&def_id).cloned() {
            Some(info) => info,
            None => return Ty::Var(self.fresh()),
        };
        let decl_fields = match &info.body {
            crate::check::registry::TypeDeclBody::Record(fs) => fs.clone(),
            _ => {
                self.errors.push(Error {
                    span: e.span,
                    kind: crate::error::ErrorKind::UnknownType {
                        name: type_name.to_string(),
                    },
                });
                return Ty::Var(self.fresh());
            }
        };

        // Instantiate the type's params with fresh tyvars; substitute through
        // each declared field type so unification produces inferences for them.
        let mut inst: Subst = HashMap::new();
        let mut result_args: Vec<Ty> = Vec::with_capacity(info.params.len());
        for p in &info.params {
            let fresh = Ty::Var(self.fresh());
            inst.insert(*p, fresh.clone());
            result_args.push(fresh);
        }
        let instantiated: Vec<(String, Ty)> = decl_fields
            .iter()
            .map(|f| (f.name.clone(), apply_subst(&f.ty, &inst)))
            .collect();

        let declared_names: std::collections::HashSet<&str> =
            instantiated.iter().map(|(n, _)| n.as_str()).collect();
        let provided_names: std::collections::HashSet<&str> =
            fields.iter().map(|f| f.name.as_str()).collect();
        for missing in declared_names.difference(&provided_names) {
            self.errors.push(Error {
                span: e.span,
                kind: crate::error::ErrorKind::MissingField {
                    type_name: type_name.to_string(),
                    field: missing.to_string(),
                },
            });
        }
        for extra in provided_names.difference(&declared_names) {
            self.errors.push(Error {
                span: e.span,
                kind: crate::error::ErrorKind::UnknownField {
                    type_name: type_name.to_string(),
                    field: extra.to_string(),
                },
            });
        }
        for kw in fields {
            let provided_ty = self.infer_expr(&kw.value);
            if let Some((_, decl_ty)) = instantiated.iter().find(|(n, _)| n == &kw.name)
                && let Err(ue) = unify(&mut self.subst, decl_ty, &provided_ty)
            {
                self.errors
                    .push(crate::check::unify_error_to_error(kw.value.span, ue));
            }
        }
        Ty::Con(def_id, result_args)
    }

    fn infer_update(&mut self, e: &Expr, value: &Expr, fields: &[crate::ast::KwArg]) -> Ty {
        let value_ty = self.infer_expr(value);
        let resolved = apply_subst(&value_ty, &self.subst);
        let (def_id, type_args) = match &resolved {
            Ty::Con(id, args) => (*id, args.clone()),
            _ => return value_ty,
        };
        let info = match self.registry.types.get(&def_id).cloned() {
            Some(info) => info,
            None => return value_ty,
        };
        let decl_fields = match &info.body {
            crate::check::registry::TypeDeclBody::Record(fs) => fs.clone(),
            _ => return value_ty,
        };

        let mut inst: Subst = HashMap::new();
        for (p, a) in info.params.iter().zip(type_args.iter()) {
            inst.insert(*p, a.clone());
        }
        let instantiated: Vec<(String, Ty)> = decl_fields
            .iter()
            .map(|f| (f.name.clone(), apply_subst(&f.ty, &inst)))
            .collect();

        let declared_names: std::collections::HashSet<&str> =
            instantiated.iter().map(|(n, _)| n.as_str()).collect();
        for kw in fields {
            if !declared_names.contains(kw.name.as_str()) {
                self.errors.push(Error {
                    span: e.span,
                    kind: crate::error::ErrorKind::UnknownField {
                        type_name: info.name.clone(),
                        field: kw.name.clone(),
                    },
                });
            }
        }
        for kw in fields {
            let provided_ty = self.infer_expr(&kw.value);
            if let Some((_, decl_ty)) = instantiated.iter().find(|(n, _)| n == &kw.name)
                && let Err(ue) = unify(&mut self.subst, decl_ty, &provided_ty)
            {
                self.errors
                    .push(crate::check::unify_error_to_error(kw.value.span, ue));
            }
        }
        Ty::Con(def_id, type_args)
    }

    fn infer_field_access(&mut self, e: &Expr, receiver: &Expr, field: &str) -> Ty {
        let recv_ty = self.infer_expr(receiver);
        let resolved = apply_subst(&recv_ty, &self.subst);
        let (parent_def_id, type_args) = match &resolved {
            Ty::Con(id, args) => (*id, args.clone()),
            _ => {
                self.errors.push(Error {
                    span: receiver.span,
                    kind: crate::error::ErrorKind::CannotAccessMember {
                        ty: format!("{resolved}"),
                        member: field.to_string(),
                    },
                });
                return Ty::Var(self.fresh());
            }
        };
        let info = match self.registry.types.get(&parent_def_id).cloned() {
            Some(info) => info,
            None => return Ty::Var(self.fresh()),
        };

        // Substitution from the type's params to the receiver's type args, used
        // to instantiate both field types and method schemes.
        let mut inst: Subst = HashMap::new();
        for (p, a) in info.params.iter().zip(type_args.iter()) {
            inst.insert(*p, a.clone());
        }

        // Field lookup (records only — sum-payload field access is later work).
        if let crate::check::registry::TypeDeclBody::Record(fields) = &info.body
            && let Some(f) = fields.iter().find(|f| f.name == field)
        {
            return apply_subst(&f.ty, &inst);
        }

        // Method lookup.
        if let Some(m) = info.methods.iter().find(|m| m.name == field) {
            let mut method_inst = inst.clone();
            for v in &m.scheme.vars {
                if !method_inst.contains_key(v) {
                    method_inst.insert(*v, Ty::Var(self.fresh()));
                }
            }
            let instantiated = apply_subst(&m.scheme.ty, &method_inst);
            if let Ty::Fun(params, ret) = instantiated {
                if params.is_empty() {
                    return *ret;
                }
                if let Err(ue) = unify(&mut self.subst, &params[0], &recv_ty) {
                    self.errors
                        .push(crate::check::unify_error_to_error(receiver.span, ue));
                }
                let rest = params[1..].to_vec();
                if rest.is_empty() {
                    return *ret;
                }
                return Ty::Fun(rest, ret);
            }
            return instantiated;
        }

        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::UnknownField {
                type_name: info.name.clone(),
                field: field.to_string(),
            },
        });
        Ty::Var(self.fresh())
    }

    fn infer_ctor_pattern(&mut self, p: &Pattern, name: &str, args: &[Pattern]) -> PatternResult {
        let ctor_id = match self.res.refs.get(&p.span).cloned() {
            Some(ResolvedName::Ctor(id)) => id,
            _ => {
                self.errors.push(Error {
                    span: p.span,
                    kind: crate::error::ErrorKind::Unresolved {
                        name: name.to_string(),
                    },
                });
                return PatternResult {
                    ty: Ty::Var(self.fresh()),
                    bindings: vec![],
                };
            }
        };
        let scheme = match self.schemes.get(&ctor_id).cloned() {
            Some(s) => s,
            None => {
                return PatternResult {
                    ty: Ty::Var(self.fresh()),
                    bindings: vec![],
                };
            }
        };
        let inst = self.instantiate(scheme);
        let (payload_tys, result_ty) = match inst {
            Ty::Fun(ps, r) => (ps, *r),
            other => (Vec::new(), other),
        };
        if payload_tys.len() != args.len() {
            self.errors.push(Error {
                span: p.span,
                kind: crate::error::ErrorKind::ArityMismatch {
                    expected: payload_tys.len(),
                    found: args.len(),
                },
            });
        }
        let mut bindings = Vec::new();
        for (pt, sub_pat) in payload_tys.iter().zip(args.iter()) {
            let sub = self.infer_pattern(sub_pat);
            if let Err(ue) = unify(&mut self.subst, pt, &sub.ty) {
                self.errors
                    .push(crate::check::unify_error_to_error(sub_pat.span, ue));
            }
            bindings.extend(sub.bindings);
        }
        PatternResult {
            ty: result_ty,
            bindings,
        }
    }

    fn infer_record_pattern(
        &mut self,
        p: &Pattern,
        type_name: &str,
        fields: &[crate::ast::FieldPat],
    ) -> PatternResult {
        let type_def_id = match self.res.refs.get(&p.span).cloned() {
            Some(ResolvedName::TopLevel(id)) => id,
            Some(ResolvedName::Ctor(id)) => match self.registry.ctor_to_type.get(&id).cloned() {
                Some(parent) => parent,
                None => {
                    return PatternResult {
                        ty: Ty::Var(self.fresh()),
                        bindings: vec![],
                    };
                }
            },
            _ => {
                self.errors.push(Error {
                    span: p.span,
                    kind: crate::error::ErrorKind::Unresolved {
                        name: type_name.to_string(),
                    },
                });
                return PatternResult {
                    ty: Ty::Var(self.fresh()),
                    bindings: vec![],
                };
            }
        };
        let info = match self.registry.types.get(&type_def_id).cloned() {
            Some(i) => i,
            None => {
                return PatternResult {
                    ty: Ty::Var(self.fresh()),
                    bindings: vec![],
                };
            }
        };
        let decl_fields = match &info.body {
            crate::check::registry::TypeDeclBody::Record(fs) => fs.clone(),
            _ => {
                self.errors.push(Error {
                    span: p.span,
                    kind: crate::error::ErrorKind::UnknownType {
                        name: type_name.to_string(),
                    },
                });
                return PatternResult {
                    ty: Ty::Var(self.fresh()),
                    bindings: vec![],
                };
            }
        };
        let mut inst: Subst = HashMap::new();
        let mut result_args: Vec<Ty> = Vec::with_capacity(info.params.len());
        for v in &info.params {
            let fresh = Ty::Var(self.fresh());
            inst.insert(*v, fresh.clone());
            result_args.push(fresh);
        }
        let instantiated: Vec<(String, Ty)> = decl_fields
            .iter()
            .map(|f| (f.name.clone(), apply_subst(&f.ty, &inst)))
            .collect();
        let mut bindings = Vec::new();
        for fp in fields {
            let sub = self.infer_pattern(&fp.pattern);
            bindings.extend(sub.bindings);
            match instantiated.iter().find(|(n, _)| n == &fp.field) {
                Some((_, decl_ty)) => {
                    if let Err(ue) = unify(&mut self.subst, decl_ty, &sub.ty) {
                        self.errors
                            .push(crate::check::unify_error_to_error(fp.pattern.span, ue));
                    }
                }
                None => self.errors.push(Error {
                    span: fp.pattern.span,
                    kind: crate::error::ErrorKind::UnknownField {
                        type_name: type_name.to_string(),
                        field: fp.field.clone(),
                    },
                }),
            }
        }
        PatternResult {
            ty: Ty::Con(type_def_id, result_args),
            bindings,
        }
    }

    fn infer_list_pattern(&mut self, p: &Pattern, items: &[Pattern]) -> PatternResult {
        let list_def_id = match self.res.defs.iter().find(|d| d.name == "List") {
            Some(d) => d.id,
            None => {
                self.errors.push(Error {
                    span: p.span,
                    kind: crate::error::ErrorKind::UnknownType {
                        name: "List".into(),
                    },
                });
                return PatternResult {
                    ty: Ty::Var(self.fresh()),
                    bindings: vec![],
                };
            }
        };
        let elem_var = self.fresh();
        let mut bindings = Vec::new();
        for item in items {
            let sub = self.infer_pattern(item);
            if let Err(ue) = unify(&mut self.subst, &Ty::Var(elem_var), &sub.ty) {
                self.errors
                    .push(crate::check::unify_error_to_error(item.span, ue));
            }
            bindings.extend(sub.bindings);
        }
        PatternResult {
            ty: Ty::Con(list_def_id, vec![Ty::Var(elem_var)]),
            bindings,
        }
    }

    pub fn infer_pattern(&mut self, p: &Pattern) -> PatternResult {
        let result = match &p.node {
            PatternKind::Wildcard => {
                let v = self.fresh();
                PatternResult {
                    ty: Ty::Var(v),
                    bindings: vec![],
                }
            }
            PatternKind::Var(_) => {
                let ty = Ty::Var(self.fresh());
                if let Some(ResolvedName::Local(lid)) = self.res.refs.get(&p.span) {
                    self.locals.insert(*lid, ty.clone());
                    PatternResult {
                        ty,
                        bindings: vec![*lid],
                    }
                } else {
                    PatternResult {
                        ty,
                        bindings: vec![],
                    }
                }
            }
            PatternKind::Lit(lit) => {
                let ty = match lit {
                    LitPat::Int(_) => Ty::Prim(PrimTy::Int),
                    LitPat::Float(_) => Ty::Prim(PrimTy::Float),
                    LitPat::Str(_) => Ty::Prim(PrimTy::String),
                };
                PatternResult {
                    ty,
                    bindings: vec![],
                }
            }
            PatternKind::Ctor { name, args } => self.infer_ctor_pattern(p, name, args),
            PatternKind::Record { type_name, fields } => {
                self.infer_record_pattern(p, type_name, fields)
            }
            PatternKind::List(items) => self.infer_list_pattern(p, items),
            _ => PatternResult {
                ty: Ty::Var(self.fresh()),
                bindings: vec![],
            },
        };
        self.record_pattern_type(p.span, result.ty.clone());
        result
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

    use crate::ast::LitPat;
    use crate::span::Spanned;

    fn pat(kind: PatternKind) -> Pattern {
        Spanned {
            span: Span::new(0, 1),
            node: kind,
        }
    }

    #[test]
    fn wildcard_pattern_has_fresh_type_var_and_no_bindings() {
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let pr = infer.infer_pattern(&pat(PatternKind::Wildcard));
        assert!(matches!(pr.ty, Ty::Var(_)));
        assert!(pr.bindings.is_empty());
    }

    #[test]
    fn int_literal_pattern_is_int_typed() {
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let pr = infer.infer_pattern(&pat(PatternKind::Lit(LitPat::Int(0))));
        assert_eq!(pr.ty, Ty::Prim(PrimTy::Int));
        assert!(pr.bindings.is_empty());
    }

    #[test]
    fn float_literal_pattern_is_float_typed() {
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let pr = infer.infer_pattern(&pat(PatternKind::Lit(LitPat::Float(1.5))));
        assert_eq!(pr.ty, Ty::Prim(PrimTy::Float));
    }

    #[test]
    fn string_literal_pattern_is_string_typed() {
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let pr = infer.infer_pattern(&pat(PatternKind::Lit(LitPat::Str("a".into()))));
        assert_eq!(pr.ty, Ty::Prim(PrimTy::String));
    }

    use crate::check::registry::{FieldInfo, TypeDeclBody, TypeDeclInfo, TypeRegistry};

    fn pat_at(start: u32, kind: PatternKind) -> Pattern {
        Spanned {
            span: Span::new(start, start + 1),
            node: kind,
        }
    }

    #[test]
    fn bare_ctor_pattern_returns_parent_type() {
        let ctor_id = DefId(10);
        let parent_id = DefId(11);
        let p = pat_at(
            0,
            PatternKind::Ctor {
                name: "None".into(),
                args: vec![],
            },
        );
        let mut res = Resolution::default();
        res.refs.insert(p.span, ResolvedName::Ctor(ctor_id));

        let mut infer = Infer::new(&res);
        let a = infer.fresh();
        infer.schemes.insert(
            ctor_id,
            Scheme {
                vars: vec![a],
                ty: Ty::Con(parent_id, vec![Ty::Var(a)]),
            },
        );

        let pr = infer.infer_pattern(&p);
        match pr.ty {
            Ty::Con(id, args) => {
                assert_eq!(id, parent_id);
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Con(parent, _), got {other:?}"),
        }
        assert!(pr.bindings.is_empty());
    }

    #[test]
    fn single_payload_ctor_pattern_binds_inner_var_to_payload_type() {
        let ctor_id = DefId(10);
        let parent_id = DefId(11);
        let local_id = LocalId(20);
        let inner = pat_at(2, PatternKind::Var("n".into()));
        let inner_span = inner.span;
        let outer = pat_at(
            0,
            PatternKind::Ctor {
                name: "Some".into(),
                args: vec![inner],
            },
        );

        let mut res = Resolution::default();
        res.refs.insert(outer.span, ResolvedName::Ctor(ctor_id));
        res.refs.insert(inner_span, ResolvedName::Local(local_id));

        let mut infer = Infer::new(&res);
        let a = infer.fresh();
        infer.schemes.insert(
            ctor_id,
            Scheme {
                vars: vec![a],
                ty: Ty::Fun(
                    vec![Ty::Var(a)],
                    Box::new(Ty::Con(parent_id, vec![Ty::Var(a)])),
                ),
            },
        );

        let pr = infer.infer_pattern(&outer);
        match &pr.ty {
            Ty::Con(id, args) => {
                assert_eq!(*id, parent_id);
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Con(parent, _), got {other:?}"),
        }
        assert_eq!(pr.bindings, vec![local_id]);
        // After unification, the bound var and the parent's first type arg
        // resolve to the same tyvar via the substitution.
        let bound = infer.locals.get(&local_id).cloned().unwrap();
        let bound = apply_subst(&bound, &infer.subst);
        let parent_arg = match &pr.ty {
            Ty::Con(_, args) => apply_subst(&args[0], &infer.subst),
            other => panic!("unexpected ty: {other:?}"),
        };
        assert_eq!(bound, parent_arg);
    }

    #[test]
    fn record_pattern_unifies_field_subpattern_types() {
        let type_id = DefId(30);
        let local_id = LocalId(40);
        let inner = pat_at(4, PatternKind::Var("xv".into()));
        let inner_span = inner.span;
        let outer = pat_at(
            0,
            PatternKind::Record {
                type_name: "Point".into(),
                fields: vec![crate::ast::FieldPat {
                    field: "x".into(),
                    pattern: inner,
                }],
            },
        );

        let mut res = Resolution::default();
        res.refs.insert(outer.span, ResolvedName::TopLevel(type_id));
        res.refs.insert(inner_span, ResolvedName::Local(local_id));

        let mut infer = Infer::new(&res);
        let mut registry = TypeRegistry::default();
        registry.types.insert(
            type_id,
            TypeDeclInfo {
                def_id: type_id,
                name: "Point".into(),
                params: vec![],
                body: TypeDeclBody::Record(vec![FieldInfo {
                    name: "x".into(),
                    ty: Ty::Prim(PrimTy::Int),
                }]),
                methods: vec![],
            },
        );
        infer.registry = registry;

        let pr = infer.infer_pattern(&outer);
        assert_eq!(pr.ty, Ty::Con(type_id, vec![]));
        assert_eq!(pr.bindings, vec![local_id]);
        // The inner Var pattern's fresh tyvar should resolve, after subst, to Int.
        let bound = infer.locals.get(&local_id).unwrap().clone();
        assert_eq!(apply_subst(&bound, &infer.subst), Ty::Prim(PrimTy::Int));
    }

    use crate::resolve::{DefInfo, DefKind};

    fn list_def(id: DefId) -> DefInfo {
        DefInfo {
            id,
            name: "List".into(),
            kind: DefKind::Type,
            span: Span::new(0, 0),
        }
    }

    #[test]
    fn list_pattern_with_int_literal_items_has_list_int_type() {
        let list_id = DefId(50);
        let outer = pat_at(
            0,
            PatternKind::List(vec![
                pat_at(2, PatternKind::Lit(LitPat::Int(1))),
                pat_at(4, PatternKind::Lit(LitPat::Int(2))),
            ]),
        );

        let mut res = Resolution::default();
        res.defs.push(list_def(list_id));

        let mut infer = Infer::new(&res);
        let pr = infer.infer_pattern(&outer);
        assert!(pr.bindings.is_empty());
        match pr.ty {
            Ty::Con(id, args) => {
                assert_eq!(id, list_id);
                assert_eq!(args.len(), 1);
                assert_eq!(apply_subst(&args[0], &infer.subst), Ty::Prim(PrimTy::Int));
            }
            other => panic!("expected Con(List, _), got {other:?}"),
        }
    }

    #[test]
    fn list_pattern_with_var_items_binds_both_to_same_element_type() {
        let list_id = DefId(50);
        let a_id = LocalId(60);
        let b_id = LocalId(61);
        let a_pat = pat_at(2, PatternKind::Var("a".into()));
        let b_pat = pat_at(4, PatternKind::Var("b".into()));
        let a_span = a_pat.span;
        let b_span = b_pat.span;
        let outer = pat_at(0, PatternKind::List(vec![a_pat, b_pat]));

        let mut res = Resolution::default();
        res.defs.push(list_def(list_id));
        res.refs.insert(a_span, ResolvedName::Local(a_id));
        res.refs.insert(b_span, ResolvedName::Local(b_id));

        let mut infer = Infer::new(&res);
        let pr = infer.infer_pattern(&outer);
        assert_eq!(pr.bindings, vec![a_id, b_id]);
        let elem_ty = match &pr.ty {
            Ty::Con(_, args) => apply_subst(&args[0], &infer.subst),
            other => panic!("expected Con(List, _), got {other:?}"),
        };
        let a_ty = apply_subst(infer.locals.get(&a_id).unwrap(), &infer.subst);
        let b_ty = apply_subst(infer.locals.get(&b_id).unwrap(), &infer.subst);
        assert_eq!(a_ty, elem_ty);
        assert_eq!(b_ty, elem_ty);
    }

    #[test]
    fn list_pattern_when_list_type_missing_emits_unknown_type_error() {
        let outer = pat_at(0, PatternKind::List(vec![]));
        let res = Resolution::default();
        let mut infer = Infer::new(&res);
        let _ = infer.infer_pattern(&outer);
        assert!(infer.errors.iter().any(|e| matches!(
            &e.kind,
            crate::error::ErrorKind::UnknownType { name } if name == "List"
        )));
    }
}
