pub mod infer;
pub mod types;
pub mod unify;

pub use types::*;

use crate::ast::{DeclKind, File};
use crate::check::infer::Infer;
use crate::check::unify::{UnifyError, unify};
use crate::error::{Error, ErrorKind};
use crate::resolve::Resolution;
use crate::span::Span;

pub fn check_file(file: &File, res: &Resolution) -> Result<Typing, Vec<Error>> {
    let mut infer = Infer::new(res);

    // Phase 1: pre-declare a fresh tyvar for every top-level value binding so
    // mutual recursion type-checks regardless of source order.
    let mut pending: Vec<(crate::resolve::DefId, &crate::ast::Expr, TyVarId)> = Vec::new();
    for decl in &file.decls {
        if let DeclKind::Binding {
            name,
            value: Some(value),
            ..
        } = &decl.node
            && let Some(def) = res.defs.iter().find(|d| &d.name == name)
        {
            let v = infer.fresh();
            infer.schemes.insert(
                def.id,
                Scheme {
                    vars: Vec::new(),
                    ty: Ty::Var(v),
                },
            );
            pending.push((def.id, value, v));
        }
    }

    // Phase 2: infer each body and unify with its pre-declared tyvar.
    for (_def_id, value, v) in &pending {
        let body_ty = infer.infer_expr(value);
        if let Err(e) = unify(&mut infer.subst, &Ty::Var(*v), &body_ty) {
            infer.errors.push(unify_error_to_error(value.span, e));
        }
    }

    // Phase 2.5: resolve each scheme through the final substitution. Done
    // after the loop because mutual recursion (`a = b; b = 1`) only pins
    // later bindings' tyvars once their own iteration completes — resolving
    // inside the phase-2 loop would freeze earlier schemes as bare tyvars.
    for (def_id, _value, v) in pending {
        let resolved = crate::check::unify::apply_subst(&Ty::Var(v), &infer.subst);
        infer.schemes.insert(
            def_id,
            Scheme {
                vars: Vec::new(),
                ty: resolved,
            },
        );
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
