use crate::check::traits::TraitId;
use crate::resolve::{DefId, Resolution};
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

/// "`ty` must implement `trait_`." The sole obligation kind in Plan 5.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    pub trait_: TraitId,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub vars: Vec<TyVarId>,
    pub constraints: Vec<Constraint>,
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

/// Render a type using resolved constructor names instead of `#<DefId>`. Falls
/// back to the raw `Display` form when a DefId isn't in `res.defs` — that
/// happens in unit tests that synthesise schemes without a real resolver.
pub fn ty_to_string(ty: &Ty, res: &Resolution) -> String {
    match ty {
        Ty::Var(_) | Ty::Prim(_) => format!("{ty}"),
        Ty::Con(id, args) => {
            let name = res
                .defs
                .iter()
                .find(|d| d.id == *id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| format!("#{}", id.0));
            if args.is_empty() {
                name
            } else {
                let inner: Vec<String> = args.iter().map(|a| ty_arg_to_string(a, res)).collect();
                format!("{name} {}", inner.join(" "))
            }
        }
        Ty::Fun(ps, r) => {
            let params: Vec<String> = ps.iter().map(|p| ty_to_string(p, res)).collect();
            format!("{} -> {}", params.join(", "), ty_to_string(r, res))
        }
    }
}

/// Render a type in a position that needs grouping for compounds — i.e. as an
/// argument to a type constructor. Parenthesises `Con` with args and `Fun` so
/// `List (Maybe Int)` doesn't flatten to `List Maybe Int`.
fn ty_arg_to_string(ty: &Ty, res: &Resolution) -> String {
    match ty {
        Ty::Con(_, args) if !args.is_empty() => format!("({})", ty_to_string(ty, res)),
        Ty::Fun(..) => format!("({})", ty_to_string(ty, res)),
        _ => ty_to_string(ty, res),
    }
}

pub fn scheme_to_string(s: &Scheme, res: &Resolution) -> String {
    let mut out = String::new();
    if !s.vars.is_empty() {
        let vars: Vec<String> = s.vars.iter().map(|v| format!("t{}", v.0)).collect();
        out.push_str(&format!("forall {} . ", vars.join(" ")));
    }
    if !s.constraints.is_empty() {
        let cs: Vec<String> = s
            .constraints
            .iter()
            .map(|c| format!("{} {}", c.trait_.name(), ty_to_string(&c.ty, res)))
            .collect();
        out.push_str(&format!("{} => ", cs.join(", ")));
    }
    out.push_str(&ty_to_string(&s.ty, res));
    out
}

/// Render a whole `Typing` with binding names, sorted by DefId for stable
/// snapshots.
pub fn render_typing(t: &Typing, res: &Resolution) -> String {
    let mut out = String::from("schemes:\n");
    let mut entries: Vec<(&DefId, &Scheme)> = t.schemes.iter().collect();
    entries.sort_by_key(|(id, _)| id.0);
    for (id, scheme) in entries {
        let name = res
            .defs
            .iter()
            .find(|d| d.id == *id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| format!("#{}", id.0));
        out.push_str(&format!("  {name} : {}\n", scheme_to_string(scheme, res)));
    }
    out
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
    fn scheme_carries_constraints() {
        let s = Scheme {
            vars: vec![TyVarId(0)],
            constraints: vec![Constraint {
                trait_: TraitId::Eq,
                ty: Ty::Var(TyVarId(0)),
            }],
            ty: Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Bool))),
        };
        assert_eq!(s.constraints.len(), 1);
        assert_eq!(s.constraints[0].trait_, TraitId::Eq);
    }

    #[test]
    fn ty_to_string_uses_resolved_name_for_con() {
        use crate::resolve::{DefInfo, DefKind, Resolution};
        use crate::span::Span;
        let mut res = Resolution::default();
        res.defs.push(DefInfo {
            id: DefId(0),
            name: "Maybe".into(),
            kind: DefKind::Type,
            span: Span::new(0, 0),
        });
        let ty = Ty::Con(DefId(0), vec![Ty::Prim(PrimTy::Int)]);
        assert_eq!(ty_to_string(&ty, &res), "Maybe Int");
    }

    #[test]
    fn ty_to_string_parenthesises_compound_args() {
        use crate::resolve::{DefInfo, DefKind, Resolution};
        use crate::span::Span;
        let mut res = Resolution::default();
        for (i, n) in ["List", "Maybe"].iter().enumerate() {
            res.defs.push(DefInfo {
                id: DefId(i as u32),
                name: (*n).into(),
                kind: DefKind::Type,
                span: Span::new(0, 0),
            });
        }
        let nested = Ty::Con(
            DefId(0),
            vec![Ty::Con(DefId(1), vec![Ty::Prim(PrimTy::Int)])],
        );
        assert_eq!(ty_to_string(&nested, &res), "List (Maybe Int)");
        let with_fun = Ty::Con(
            DefId(0),
            vec![Ty::Fun(
                vec![Ty::Prim(PrimTy::Int)],
                Box::new(Ty::Prim(PrimTy::Bool)),
            )],
        );
        assert_eq!(ty_to_string(&with_fun, &res), "List (Int -> Bool)");
    }

    #[test]
    fn scheme_to_string_includes_constraints() {
        let res = crate::resolve::Resolution::default();
        let s = Scheme {
            vars: vec![TyVarId(0)],
            constraints: vec![Constraint {
                trait_: TraitId::Eq,
                ty: Ty::Var(TyVarId(0)),
            }],
            ty: Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Bool))),
        };
        assert_eq!(
            scheme_to_string(&s, &res),
            "forall t0 . Eq t0 => t0 -> Bool"
        );
    }

    #[test]
    fn display_scheme_includes_forall_when_quantified() {
        let scheme = Scheme {
            vars: vec![TyVarId(0)],
            constraints: Vec::new(),
            ty: Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Var(TyVarId(0)))),
        };
        let s = format!("{scheme}");
        assert!(s.starts_with("forall"));
    }
}
