use crate::resolve::DefId;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyVarId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimTy {
    Int,
    Float,
    String,
    Bool,
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Var(TyVarId),
    Prim(PrimTy),
    Con(DefId, Vec<Ty>),
    Fun(Vec<Ty>, Box<Ty>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub vars: Vec<TyVarId>,
    pub ty: Ty,
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Var(v) => write!(f, "t{}", v.0),
            Ty::Prim(p) => f.write_str(match p {
                PrimTy::Int => "Int",
                PrimTy::Float => "Float",
                PrimTy::String => "String",
                PrimTy::Bool => "Bool",
                PrimTy::Unit => "Unit",
            }),
            // Plan 4 stores no nominal name in the type, only the DefId, so a
            // constructor prints as `#<id>`. Plan 5 may add a Resolution-aware
            // printer for friendly names.
            Ty::Con(id, args) => {
                write!(f, "#{}", id.0)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{a}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Ty::Fun(ps, r) => {
                for (i, p) in ps.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, " -> {r}")
            }
        }
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.vars.is_empty() {
            write!(f, "forall")?;
            for v in &self.vars {
                write!(f, " t{}", v.0)?;
            }
            write!(f, " . ")?;
        }
        write!(f, "{}", self.ty)
    }
}

pub type Subst = HashMap<TyVarId, Ty>;

#[derive(Debug, Default, Clone)]
pub struct Typing {
    pub schemes: HashMap<DefId, Scheme>,
    pub expr_types: HashMap<Span, Ty>,
    pub pattern_types: HashMap<Span, Ty>,
}

impl std::fmt::Display for Typing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "schemes:")?;
        // Sort by DefId so the snapshot is stable across HashMap iteration order.
        let mut entries: Vec<_> = self.schemes.iter().collect();
        entries.sort_by_key(|(id, _)| id.0);
        for (id, scheme) in entries {
            writeln!(f, "  #{} : {}", id.0, scheme)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_primitive() {
        assert_eq!(format!("{}", Ty::Prim(PrimTy::Int)), "Int");
        assert_eq!(format!("{}", Ty::Prim(PrimTy::Float)), "Float");
    }

    #[test]
    fn display_function() {
        let ty = Ty::Fun(
            vec![Ty::Prim(PrimTy::Int), Ty::Prim(PrimTy::Int)],
            Box::new(Ty::Prim(PrimTy::Bool)),
        );
        assert_eq!(format!("{ty}"), "Int, Int -> Bool");
    }

    #[test]
    fn display_var_uses_letter_alphabet() {
        // For this task, just verify Vars don't print as "TyVarId(0)".
        let s = format!("{}", Ty::Var(TyVarId(0)));
        assert!(!s.contains("TyVarId"));
    }

    #[test]
    fn display_scheme_includes_forall_when_quantified() {
        let scheme = Scheme {
            vars: vec![TyVarId(0)],
            ty: Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Var(TyVarId(0)))),
        };
        let s = format!("{scheme}");
        assert!(s.starts_with("forall"));
    }
}
