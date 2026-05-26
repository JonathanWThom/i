use crate::ast::{MatchArm, PatternKind};
use crate::check::registry::{TypeDeclBody, TypeRegistry};
use crate::check::types::Ty;
use std::collections::HashSet;

pub enum Coverage {
    Exhaustive,
    Missing(Vec<String>),
}

pub fn check_arms(scrutinee: &Ty, arms: &[MatchArm], registry: &TypeRegistry) -> Coverage {
    if arms
        .iter()
        .any(|a| matches!(a.pattern.node, PatternKind::Wildcard | PatternKind::Var(_)))
    {
        return Coverage::Exhaustive;
    }
    // Plan 4 only flags missing variants on sum types. Primitives (no
    // wildcard means a runtime no-match, caught later by the evaluator),
    // unresolved types, and records-without-variants all fall through to
    // Exhaustive here. Refine when totality lands in Plan 7.
    let Ty::Con(parent, _) = scrutinee else {
        return Coverage::Exhaustive;
    };
    let Some(info) = registry.types.get(parent) else {
        return Coverage::Exhaustive;
    };
    let TypeDeclBody::Sum(variants) = &info.body else {
        return Coverage::Exhaustive;
    };
    let covered: HashSet<String> = arms
        .iter()
        .filter_map(|a| match &a.pattern.node {
            PatternKind::Ctor { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let missing: Vec<String> = variants
        .iter()
        .map(|v| v.name.clone())
        .filter(|n| !covered.contains(n))
        .collect();
    if missing.is_empty() {
        Coverage::Exhaustive
    } else {
        Coverage::Missing(missing)
    }
}
