pub mod exhaust;
pub mod infer;
pub mod registry;
pub mod traits;
pub mod types;
pub mod unify;

pub use types::*;

use crate::ast::{
    BlockItem, Decl, DeclKind, Expr, ExprKind, File, KwArg, MatchArm, Pattern, PatternKind, Type,
    TypeBody, TypeMember, VariantBody,
};
use crate::check::infer::Infer;
use crate::check::registry::{
    FieldInfo, ImplInfo, MethodInfo, PayloadShape, TypeDeclBody, TypeDeclInfo, VariantInfo, head_of,
};
use crate::check::traits::{TraitId, seed_builtin_impls};
use crate::check::unify::{UnifyError, apply_subst, unify};
use crate::error::{Error, ErrorKind};
use crate::resolve::{DefId, DefKind, Resolution, ResolvedName};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

pub fn check_file(file: &File, res: &Resolution) -> Result<Typing, Vec<Error>> {
    let mut infer = Infer::new(res);
    build_registry(file, &mut infer);

    // Index sig-only bindings by name so multi-line `x : T` / `x = v` pairs can
    // be merged. Inline `x : T = v` keeps its annotation directly on the value
    // binding; either source wins.
    let mut sig_by_name: HashMap<&str, &Type> = HashMap::new();
    for decl in &file.decls {
        if let DeclKind::Binding {
            name,
            ty: Some(t),
            value: None,
        } = &decl.node
        {
            sig_by_name.entry(name.as_str()).or_insert(t);
        }
    }

    // Each top-level value binding paired with its DefId and (optional) annotation.
    let bindings: Vec<(DefId, &Expr, Option<&Type>)> = file
        .decls
        .iter()
        .filter_map(|decl| {
            if let DeclKind::Binding {
                name,
                ty,
                value: Some(value),
            } = &decl.node
            {
                let def = res.defs.iter().find(|d| &d.name == name)?;
                let annot = ty
                    .as_ref()
                    .or_else(|| sig_by_name.get(name.as_str()).copied());
                Some((def.id, value, annot))
            } else {
                None
            }
        })
        .collect();

    // Build dependency graph: binding i depends on binding j if i's body
    // references j (as a top-level Var). SCCs let later bindings see
    // already-generalised earlier ones; mutual recursion clusters together
    // and stays monomorphic for the shared tyvars.
    let idx_of: HashMap<DefId, usize> = bindings
        .iter()
        .enumerate()
        .map(|(i, (id, _, _))| (*id, i))
        .collect();
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); bindings.len()];
    for (i, (_, value, _)) in bindings.iter().enumerate() {
        let mut seen: HashSet<DefId> = HashSet::new();
        collect_top_level_refs(value, res, &mut seen);
        let mut targets: Vec<usize> = seen
            .into_iter()
            .filter_map(|id| idx_of.get(&id).copied())
            .filter(|&j| j != i)
            .collect();
        targets.sort_unstable();
        deps[i] = targets;
    }

    let sccs = tarjan_scc(&deps);

    // Process SCCs in topological order (Tarjan's returns that natively).
    for scc in &sccs {
        let scc_def_ids: HashSet<DefId> = scc.iter().map(|&i| bindings[i].0).collect();

        // Pre-declare fresh tyvar for each binding in the SCC, applying user
        // annotations *before* inference so recursive uses see the pinned type.
        let mut tyvars: Vec<TyVarId> = Vec::with_capacity(scc.len());
        for &i in scc {
            let (def_id, _, annot) = bindings[i];
            let v = infer.fresh();
            tyvars.push(v);
            infer.schemes.insert(
                def_id,
                Scheme {
                    vars: Vec::new(),
                    constraints: Vec::new(),
                    ty: Ty::Var(v),
                },
            );
            if let Some(t) = annot {
                let ann_ty = infer.lower_type(t);
                if let Err(e) = unify(&mut infer.subst, &Ty::Var(v), &ann_ty) {
                    infer.errors.push(unify_error_to_error(t.span, e));
                }
            }
        }

        // Infer each binding's body and unify with its pre-declared tyvar.
        for (idx, &i) in scc.iter().enumerate() {
            let (_, value, _) = bindings[i];
            let v = tyvars[idx];
            // If this binding's tyvar already resolves to a function type —
            // via a user annotation, or via an earlier member of a recursive
            // group that constrained it — push the expected parameter types
            // into the lambda before checking its body. Otherwise member access
            // on a parameter sees only an unresolved tyvar (the constraint would
            // unify too late). Equivalent types either way; this just resolves
            // them eagerly enough for member access.
            let expected = apply_subst(&Ty::Var(v), &infer.subst);
            let body_ty = match (&value.node, &expected) {
                (ExprKind::Lambda { params, body }, Ty::Fun(param_tys, _))
                    if params.len() == param_tys.len() =>
                {
                    let param_tys = param_tys.clone();
                    infer.infer_lambda_checked(value, params, body, &param_tys)
                }
                _ => infer.infer_expr(value),
            };
            if let Err(e) = unify(&mut infer.subst, &Ty::Var(v), &body_ty) {
                infer.errors.push(unify_error_to_error(value.span, e));
            }
        }

        // Resolve each scheme through the current substitution.
        for (idx, &i) in scc.iter().enumerate() {
            let def_id = bindings[i].0;
            let v = tyvars[idx];
            let resolved = apply_subst(&Ty::Var(v), &infer.subst);
            infer.schemes.insert(
                def_id,
                Scheme {
                    vars: Vec::new(),
                    constraints: Vec::new(),
                    ty: resolved,
                },
            );
        }

        // Generalise: a tyvar in this SCC's scheme.ty is bound here only if it
        // doesn't escape into another already-finalised scheme's free vars OR
        // into another scheme in the same SCC (no polymorphic recursion across
        // mutually-recursive group members).
        let mut base_env_free: HashSet<TyVarId> = HashSet::new();
        for (other_id, other_scheme) in &infer.schemes {
            if scc_def_ids.contains(other_id) {
                continue;
            }
            let resolved = apply_subst(&other_scheme.ty, &infer.subst);
            let mut free = free_vars(&resolved);
            for v in &other_scheme.vars {
                free.remove(v);
            }
            base_env_free.extend(free);
        }

        let scc_resolved: Vec<Ty> = scc
            .iter()
            .map(|&i| infer.schemes[&bindings[i].0].ty.clone())
            .collect();

        for (idx, &i) in scc.iter().enumerate() {
            let def_id = bindings[i].0;
            let mut env_free = base_env_free.clone();
            for (other_idx, other_ty) in scc_resolved.iter().enumerate() {
                if other_idx == idx {
                    continue;
                }
                env_free.extend(free_vars(other_ty));
            }
            let mine: Vec<TyVarId> = free_vars(&scc_resolved[idx])
                .into_iter()
                .filter(|v| !env_free.contains(v))
                .collect();
            if let Some(scheme) = infer.schemes.get_mut(&def_id) {
                scheme.vars = mine;
            }
        }
    }

    if infer.errors.is_empty() {
        Ok(infer.into_typing())
    } else {
        Err(std::mem::take(&mut infer.errors))
    }
}

pub(crate) fn unify_error_to_error(span: Span, e: UnifyError) -> Error {
    let kind = match e {
        UnifyError::Mismatch { left, right } => ErrorKind::TypeMismatch {
            expected: format!("{:?}", left),
            found: format!("{:?}", right),
        },
        UnifyError::Occurs(v) => ErrorKind::OccursCheck {
            var: format!("{:?}", v),
        },
        UnifyError::Arity { expected, found } => ErrorKind::ArityMismatch { expected, found },
    };
    Error { span, kind }
}

fn free_vars(ty: &Ty) -> HashSet<TyVarId> {
    fn walk(ty: &Ty, out: &mut HashSet<TyVarId>) {
        match ty {
            Ty::Var(v) => {
                out.insert(*v);
            }
            Ty::Prim(_) => {}
            Ty::Con(_, args) => args.iter().for_each(|a| walk(a, out)),
            Ty::Fun(ps, r) => {
                ps.iter().for_each(|p| walk(p, out));
                walk(r, out);
            }
        }
    }
    let mut out = HashSet::new();
    walk(ty, &mut out);
    out
}

fn collect_top_level_refs(e: &Expr, res: &Resolution, out: &mut HashSet<DefId>) {
    if let Some(ResolvedName::TopLevel(id)) = res.refs.get(&e.span) {
        out.insert(*id);
    }
    match &e.node {
        ExprKind::IntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::StringLit(_)
        | ExprKind::Var(_)
        | ExprKind::Ctor(_) => {}
        ExprKind::Paren(inner) => collect_top_level_refs(inner, res, out),
        ExprKind::BinOp { lhs, rhs, .. } => {
            collect_top_level_refs(lhs, res, out);
            collect_top_level_refs(rhs, res, out);
        }
        ExprKind::UnaryOp { expr, .. } => collect_top_level_refs(expr, res, out),
        ExprKind::List(items) => items
            .iter()
            .for_each(|i| collect_top_level_refs(i, res, out)),
        ExprKind::Lambda { body, .. } => collect_top_level_refs(body, res, out),
        ExprKind::Construct { fields, .. } => fields
            .iter()
            .for_each(|kw: &KwArg| collect_top_level_refs(&kw.value, res, out)),
        ExprKind::Update { value, fields } => {
            collect_top_level_refs(value, res, out);
            fields
                .iter()
                .for_each(|kw: &KwArg| collect_top_level_refs(&kw.value, res, out));
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_top_level_refs(scrutinee, res, out);
            arms.iter().for_each(|arm: &MatchArm| {
                collect_pattern_refs(&arm.pattern, res, out);
                collect_top_level_refs(&arm.body, res, out);
            });
        }
        ExprKind::Call { func, args } => {
            collect_top_level_refs(func, res, out);
            args.iter()
                .for_each(|a| collect_top_level_refs(a, res, out));
        }
        ExprKind::MethodCall { receiver, .. } => collect_top_level_refs(receiver, res, out),
        ExprKind::FieldAccess { receiver, .. } => collect_top_level_refs(receiver, res, out),
        ExprKind::Bang(inner) | ExprKind::Question(inner) => {
            collect_top_level_refs(inner, res, out)
        }
        ExprKind::Block(items) => items.iter().for_each(|item| match item {
            BlockItem::Expr(e) => collect_top_level_refs(e, res, out),
            BlockItem::Binding(decl) => collect_decl_refs(decl, res, out),
        }),
    }
}

fn collect_pattern_refs(p: &Pattern, res: &Resolution, out: &mut HashSet<DefId>) {
    if let Some(ResolvedName::TopLevel(id)) = res.refs.get(&p.span) {
        out.insert(*id);
    }
    match &p.node {
        PatternKind::Wildcard | PatternKind::Lit(_) | PatternKind::Var(_) => {}
        PatternKind::Ctor { args, .. } => {
            args.iter().for_each(|a| collect_pattern_refs(a, res, out))
        }
        PatternKind::Tuple(items) | PatternKind::List(items) => {
            items.iter().for_each(|i| collect_pattern_refs(i, res, out))
        }
        PatternKind::Record { fields, .. } => fields
            .iter()
            .for_each(|fp| collect_pattern_refs(&fp.pattern, res, out)),
    }
}

fn collect_decl_refs(decl: &Decl, res: &Resolution, out: &mut HashSet<DefId>) {
    if let DeclKind::Binding { value: Some(v), .. } = &decl.node {
        collect_top_level_refs(v, res, out);
    }
}

fn tarjan_scc(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut idx_counter: usize = 0;
    let mut stack: Vec<usize> = Vec::new();
    let mut on_stack = vec![false; n];
    let mut indices: Vec<Option<usize>> = vec![None; n];
    let mut lowlinks = vec![0usize; n];
    let mut result: Vec<Vec<usize>> = Vec::new();

    // Iterative DFS to avoid stack overflow on pathologically deep graphs.
    for v_start in 0..n {
        if indices[v_start].is_some() {
            continue;
        }
        // call-stack: (node, child-index-being-processed)
        let mut call_stack: Vec<(usize, usize)> = Vec::new();
        indices[v_start] = Some(idx_counter);
        lowlinks[v_start] = idx_counter;
        idx_counter += 1;
        stack.push(v_start);
        on_stack[v_start] = true;
        call_stack.push((v_start, 0));

        while let Some(&(v, child_i)) = call_stack.last() {
            if child_i < adj[v].len() {
                let w = adj[v][child_i];
                call_stack.last_mut().unwrap().1 += 1;
                if indices[w].is_none() {
                    indices[w] = Some(idx_counter);
                    lowlinks[w] = idx_counter;
                    idx_counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call_stack.push((w, 0));
                } else if on_stack[w] {
                    lowlinks[v] = lowlinks[v].min(indices[w].unwrap());
                }
            } else {
                // Finished children. Pop and propagate lowlink to parent.
                call_stack.pop();
                if lowlinks[v] == indices[v].unwrap() {
                    let mut scc = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    result.push(scc);
                }
                if let Some(&(parent, _)) = call_stack.last() {
                    lowlinks[parent] = lowlinks[parent].min(lowlinks[v]);
                }
            }
        }
    }

    result
}

fn build_registry(file: &File, infer: &mut Infer) {
    for decl in &file.decls {
        let DeclKind::TypeDecl { name, params, body } = &decl.node else {
            continue;
        };
        let Some(type_def) = infer
            .res
            .defs
            .iter()
            .find(|d| &d.name == name && matches!(d.kind, DefKind::Type))
        else {
            continue;
        };
        let type_def_id = type_def.id;

        // Allocate one fresh tyvar per type parameter so every occurrence of
        // the same param name inside this declaration unifies to the same var.
        let mut param_ctx: HashMap<String, TyVarId> = HashMap::new();
        let mut param_tyvars: Vec<TyVarId> = Vec::with_capacity(params.len());
        for p in params {
            let v = infer.fresh();
            param_ctx.insert(p.clone(), v);
            param_tyvars.push(v);
        }
        let parent_ty = Ty::Con(
            type_def_id,
            param_tyvars.iter().map(|&v| Ty::Var(v)).collect(),
        );

        let body_info = match body {
            TypeBody::Newtype(t) => TypeDeclBody::Newtype(infer.lower_type_in_scope(t, &param_ctx)),
            TypeBody::Block(members) => {
                let mut fields: Vec<FieldInfo> = Vec::new();
                let mut variants: Vec<VariantInfo> = Vec::new();
                for m in members {
                    match m {
                        TypeMember::Field { name: fname, ty } => {
                            let t = infer.lower_type_in_scope(ty, &param_ctx);
                            fields.push(FieldInfo {
                                name: fname.clone(),
                                ty: t,
                            });
                        }
                        TypeMember::Variant {
                            name: vname,
                            body: vbody,
                        } => {
                            let payload = match vbody {
                                VariantBody::Bare => PayloadShape::Bare,
                                VariantBody::Single(t) => {
                                    PayloadShape::Single(infer.lower_type_in_scope(t, &param_ctx))
                                }
                                VariantBody::Fields(sub) => {
                                    let mut sub_fields = Vec::new();
                                    for sm in sub {
                                        if let TypeMember::Field {
                                            name: sfname,
                                            ty: sfty,
                                        } = sm
                                        {
                                            let t = infer.lower_type_in_scope(sfty, &param_ctx);
                                            sub_fields.push(FieldInfo {
                                                name: sfname.clone(),
                                                ty: t,
                                            });
                                        }
                                    }
                                    PayloadShape::Record(sub_fields)
                                }
                            };
                            let ctor_def_id = infer
                                .res
                                .defs
                                .iter()
                                .find(|d| {
                                    &d.name == vname && matches!(d.kind, DefKind::Ctor { .. })
                                })
                                .map(|d| d.id);
                            if let Some(ctor_id) = ctor_def_id {
                                variants.push(VariantInfo {
                                    name: vname.clone(),
                                    ctor_def_id: ctor_id,
                                    payload,
                                    parent: type_def_id,
                                });
                            }
                        }
                        TypeMember::Method(_) => {
                            // Method registration is Task 14/15 territory.
                        }
                    }
                }
                if !fields.is_empty() && !variants.is_empty() {
                    infer.errors.push(Error {
                        span: decl.span,
                        kind: ErrorKind::MixedFieldsAndVariants { name: name.clone() },
                    });
                    continue;
                }
                if !variants.is_empty() {
                    TypeDeclBody::Sum(variants)
                } else {
                    TypeDeclBody::Record(fields)
                }
            }
        };

        // Register constructor schemes for sum variants.
        if let TypeDeclBody::Sum(variants) = &body_info {
            for variant in variants {
                let scheme_ty = match &variant.payload {
                    PayloadShape::Bare => parent_ty.clone(),
                    PayloadShape::Single(ty) => {
                        Ty::Fun(vec![ty.clone()], Box::new(parent_ty.clone()))
                    }
                    PayloadShape::Record(fs) => Ty::Fun(
                        fs.iter().map(|f| f.ty.clone()).collect(),
                        Box::new(parent_ty.clone()),
                    ),
                };
                infer.schemes.insert(
                    variant.ctor_def_id,
                    Scheme {
                        vars: param_tyvars.clone(),
                        constraints: Vec::new(),
                        ty: scheme_ty,
                    },
                );
                infer
                    .registry
                    .ctor_to_type
                    .insert(variant.ctor_def_id, type_def_id);
            }
        }

        let info = TypeDeclInfo {
            def_id: type_def_id,
            name: name.clone(),
            params: param_tyvars,
            body: body_info,
            methods: Vec::new(),
        };
        infer.registry.types.insert(type_def_id, info);
    }

    // Second pass: methods. Run after every type is in the registry so a
    // method body can field-access another type that's declared later.
    for decl in &file.decls {
        let DeclKind::TypeDecl {
            name,
            body: TypeBody::Block(members),
            ..
        } = &decl.node
        else {
            continue;
        };
        let Some(type_def) = infer
            .res
            .defs
            .iter()
            .find(|d| &d.name == name && matches!(d.kind, DefKind::Type))
        else {
            continue;
        };
        let parent_def_id = type_def.id;
        let parent_params = infer
            .registry
            .types
            .get(&parent_def_id)
            .map(|i| i.params.clone())
            .unwrap_or_default();
        let parent_ty = Ty::Con(
            parent_def_id,
            parent_params.iter().map(|&v| Ty::Var(v)).collect(),
        );

        for m in members {
            let TypeMember::Method(method_decl) = m else {
                continue;
            };
            let DeclKind::Binding {
                name: mname,
                value: Some(body),
                ..
            } = &method_decl.node
            else {
                continue;
            };
            // Resolver records self's LocalId at method_decl.span.
            let self_local = match infer.res.refs.get(&method_decl.span) {
                Some(ResolvedName::Local(id)) => *id,
                _ => continue,
            };
            infer.locals.insert(self_local, parent_ty.clone());
            let body_ty = infer.infer_expr(body);
            let result_resolved = apply_subst(&body_ty, &infer.subst);
            let fun_ty = Ty::Fun(vec![parent_ty.clone()], Box::new(result_resolved.clone()));

            // Generalise: bind parent params + any other free tyvars in the
            // method's type that don't escape into env (env_free is empty here
            // because methods get processed before top-level SCCs).
            let mut quantified: Vec<TyVarId> = parent_params.clone();
            for v in free_vars(&fun_ty) {
                if !quantified.contains(&v) {
                    quantified.push(v);
                }
            }
            let scheme = Scheme {
                vars: quantified,
                constraints: Vec::new(),
                ty: fun_ty,
            };
            if let Some(info) = infer.registry.types.get_mut(&parent_def_id) {
                info.methods.push(MethodInfo {
                    name: mname.clone(),
                    scheme,
                });
            }
        }
    }

    // Each `impl Trait Type` registers (TraitId, head) in the impl table after
    // coherence checks. Method bodies aren't checked against trait signatures
    // in Plan 5 — only the method set is.
    for decl in &file.decls {
        let DeclKind::ImplDecl {
            trait_name,
            target,
            methods,
        } = &decl.node
        else {
            continue;
        };
        let Some(trait_) = TraitId::from_name(trait_name) else {
            infer.errors.push(Error {
                span: decl.span,
                kind: ErrorKind::UnknownTrait {
                    name: trait_name.clone(),
                },
            });
            continue;
        };
        let target_ty = infer.lower_type(target);
        // Vars and Fun have no head. For a named-but-unknown type, lower_type
        // already pushed `Unresolved`, so we just skip rather than double-report.
        let Some(head) = head_of(&target_ty) else {
            continue;
        };

        if infer.registry.impls.contains_key(&(trait_, head)) {
            infer.errors.push(Error {
                span: decl.span,
                kind: ErrorKind::DuplicateImpl {
                    trait_name: trait_.name().to_string(),
                    ty: format!("{target_ty}"),
                },
            });
            continue;
        }

        let required = trait_.method_names();
        let provided: Vec<&str> = methods
            .iter()
            .filter_map(|m| match &m.node {
                DeclKind::Binding {
                    name,
                    value: Some(_),
                    ..
                } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        for req in required {
            if !provided.contains(req) {
                infer.errors.push(Error {
                    span: decl.span,
                    kind: ErrorKind::MissingMethod {
                        trait_name: trait_.name().to_string(),
                        method: req.to_string(),
                    },
                });
            }
        }
        for p in &provided {
            if !required.contains(p) {
                infer.errors.push(Error {
                    span: decl.span,
                    kind: ErrorKind::UnknownMethod {
                        trait_name: trait_.name().to_string(),
                        method: p.to_string(),
                    },
                });
            }
        }

        infer
            .registry
            .impls
            .insert((trait_, head), ImplInfo { trait_, head });
    }

    seed_builtin_impls(&mut infer.registry);
}
