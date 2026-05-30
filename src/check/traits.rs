use crate::ast::{BinOp, UnaryOp};
use crate::check::registry::{ImplInfo, TypeHead, TypeRegistry};
use crate::check::types::PrimTy;

/// The built-in operator traits. Intrinsic in Plan 5 — there is no prelude
/// declaring them yet (Plan 9). Operators are the only thing that names a
/// trait in this plan, so this small closed set is the whole trait universe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitId {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Neg,
    Eq,
    Ord,
    Concat,
}

impl TraitId {
    /// The trait an infix operator dispatches to. `and`/`or`/`xor` are Bool
    /// functions, not trait operators, so they return `None`.
    pub fn of_binop(op: &BinOp) -> Option<TraitId> {
        Some(match op {
            BinOp::Add => TraitId::Add,
            BinOp::Sub => TraitId::Sub,
            BinOp::Mul => TraitId::Mul,
            BinOp::Div => TraitId::Div,
            BinOp::Pow => TraitId::Pow,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => TraitId::Ord,
            BinOp::Eq | BinOp::Ne => TraitId::Eq,
            BinOp::Concat => TraitId::Concat,
            BinOp::And | BinOp::Or | BinOp::Xor => return None,
        })
    }

    pub fn of_unaryop(op: &UnaryOp) -> Option<TraitId> {
        match op {
            UnaryOp::Neg => Some(TraitId::Neg),
            UnaryOp::Not => None,
        }
    }

    pub fn result_is_bool(&self) -> bool {
        matches!(self, TraitId::Eq | TraitId::Ord)
    }

    pub fn name(&self) -> &'static str {
        match self {
            TraitId::Add => "Add",
            TraitId::Sub => "Sub",
            TraitId::Mul => "Mul",
            TraitId::Div => "Div",
            TraitId::Pow => "Pow",
            TraitId::Neg => "Neg",
            TraitId::Eq => "Eq",
            TraitId::Ord => "Ord",
            TraitId::Concat => "Concat",
        }
    }

    pub fn from_name(name: &str) -> Option<TraitId> {
        Some(match name {
            "Add" => TraitId::Add,
            "Sub" => TraitId::Sub,
            "Mul" => TraitId::Mul,
            "Div" => TraitId::Div,
            "Pow" => TraitId::Pow,
            "Neg" => TraitId::Neg,
            "Eq" => TraitId::Eq,
            "Ord" => TraitId::Ord,
            "Concat" => TraitId::Concat,
            _ => return None,
        })
    }

    pub fn method_names(&self) -> &'static [&'static str] {
        match self {
            TraitId::Add => &["add"],
            TraitId::Sub => &["sub"],
            TraitId::Mul => &["mul"],
            TraitId::Div => &["div"],
            TraitId::Pow => &["pow"],
            TraitId::Neg => &["neg"],
            TraitId::Eq => &["eq", "ne"],
            TraitId::Ord => &["lt", "le", "gt", "ge"],
            TraitId::Concat => &["concat"],
        }
    }
}

/// Synthesises the impls that a future `prelude.i` (Plan 9) will provide in
/// source: Eq/Ord on every primitive, numeric traits on Int and Float,
/// Concat on String.
pub fn seed_builtin_impls(reg: &mut TypeRegistry) {
    let eq_ord: &[PrimTy] = &[
        PrimTy::Int,
        PrimTy::Float,
        PrimTy::String,
        PrimTy::Bool,
        PrimTy::Unit,
    ];
    let numeric: &[PrimTy] = &[PrimTy::Int, PrimTy::Float];

    let add = |t: TraitId, p: PrimTy, reg: &mut TypeRegistry| {
        let head = TypeHead::Prim(p);
        reg.impls.insert((t, head), ImplInfo { trait_: t, head });
    };
    for &p in eq_ord {
        add(TraitId::Eq, p, reg);
        add(TraitId::Ord, p, reg);
    }
    for &p in numeric {
        for t in [
            TraitId::Add,
            TraitId::Sub,
            TraitId::Mul,
            TraitId::Div,
            TraitId::Pow,
            TraitId::Neg,
        ] {
            add(t, p, reg);
        }
    }
    add(TraitId::Concat, PrimTy::String, reg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binop_maps_to_trait() {
        assert_eq!(TraitId::of_binop(&BinOp::Add), Some(TraitId::Add));
        assert_eq!(TraitId::of_binop(&BinOp::Lt), Some(TraitId::Ord));
        assert_eq!(TraitId::of_binop(&BinOp::Eq), Some(TraitId::Eq));
        assert_eq!(TraitId::of_binop(&BinOp::Concat), Some(TraitId::Concat));
        // and/or/xor are Std.Bool functions, not trait operators (spec §11).
        assert_eq!(TraitId::of_binop(&BinOp::And), None);
    }

    #[test]
    fn result_is_bool_for_comparisons_else_operand() {
        assert!(TraitId::Eq.result_is_bool());
        assert!(TraitId::Ord.result_is_bool());
        assert!(!TraitId::Add.result_is_bool());
    }

    #[test]
    fn name_and_methods_are_known() {
        assert_eq!(TraitId::Eq.name(), "Eq");
        assert_eq!(TraitId::Eq.method_names(), &["eq", "ne"]);
        assert_eq!(TraitId::Add.method_names(), &["add"]);
    }

    #[test]
    fn trait_id_parses_from_name() {
        assert_eq!(TraitId::from_name("Ord"), Some(TraitId::Ord));
        assert_eq!(TraitId::from_name("Nope"), None);
    }

    #[test]
    fn builtin_impls_cover_primitive_arithmetic_and_eq() {
        use crate::check::registry::{TypeHead, TypeRegistry};
        use crate::check::types::PrimTy;
        let mut reg = TypeRegistry::default();
        seed_builtin_impls(&mut reg);
        assert!(
            reg.impls
                .contains_key(&(TraitId::Add, TypeHead::Prim(PrimTy::Int)))
        );
        assert!(
            reg.impls
                .contains_key(&(TraitId::Add, TypeHead::Prim(PrimTy::Float)))
        );
        assert!(
            reg.impls
                .contains_key(&(TraitId::Eq, TypeHead::Prim(PrimTy::Int)))
        );
        assert!(
            reg.impls
                .contains_key(&(TraitId::Ord, TypeHead::Prim(PrimTy::Float)))
        );
        assert!(
            reg.impls
                .contains_key(&(TraitId::Concat, TypeHead::Prim(PrimTy::String)))
        );
        // No arithmetic on String.
        assert!(
            !reg.impls
                .contains_key(&(TraitId::Add, TypeHead::Prim(PrimTy::String)))
        );
    }
}
