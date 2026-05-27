use crate::ast::{BinOp, UnaryOp};

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
}
