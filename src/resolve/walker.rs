use super::scope::ScopeStack;
use super::types::{DefKind, Resolution, ResolvedName};
use crate::ast::{Decl, DeclKind, Expr, ExprKind, File, Pattern, PatternKind};
use crate::error::{Error, ErrorKind};
use crate::span::Span;

pub(super) struct Walker<'a> {
    res: &'a mut Resolution,
    errors: &'a mut Vec<Error>,
    scope: ScopeStack,
}

impl<'a> Walker<'a> {
    fn walk_file(&mut self, file: &File) {
        for decl in &file.decls {
            self.walk_decl(decl);
        }
    }

    fn walk_decl(&mut self, decl: &Decl) {
        match &decl.node {
            DeclKind::Binding { ty, value, .. } => {
                if let Some(t) = ty {
                    self.walk_type(t);
                }
                if let Some(v) = value {
                    self.walk_expr(v);
                }
            }
            DeclKind::TypeDecl { body, .. } => self.walk_type_body(body),
            DeclKind::TraitDecl { methods, .. } | DeclKind::ImplDecl { methods, .. } => {
                for m in methods {
                    self.walk_method(m);
                }
            }
            DeclKind::Use { .. } => {}
        }
    }

    fn walk_type_body(&mut self, body: &crate::ast::TypeBody) {
        use crate::ast::TypeBody;
        match body {
            TypeBody::Newtype(t) => self.walk_type(t),
            TypeBody::Block(members) => {
                for m in members {
                    self.walk_type_member(m);
                }
            }
        }
    }

    fn walk_type_member(&mut self, m: &crate::ast::TypeMember) {
        use crate::ast::{TypeMember, VariantBody};
        match m {
            TypeMember::Field { ty, .. } => self.walk_type(ty),
            TypeMember::Method(d) => self.walk_method(d),
            TypeMember::Variant { body, .. } => match body {
                VariantBody::Bare => {}
                VariantBody::Single(t) => self.walk_type(t),
                VariantBody::Fields(sub) => {
                    for inner in sub {
                        self.walk_type_member(inner);
                    }
                }
            },
        }
    }

    fn walk_type(&mut self, t: &crate::ast::Type) {
        use crate::ast::{EffectRow, TypeKind};
        match &t.node {
            TypeKind::Var(_) => {}
            TypeKind::Named { name, args } => {
                self.resolve_type_name(name, t.span);
                for a in args {
                    self.walk_type(a);
                }
            }
            TypeKind::Function {
                params,
                effect,
                result,
            } => {
                for p in params {
                    self.walk_type(p);
                }
                if let Some(EffectRow::Named(names)) = effect {
                    for n in names {
                        self.resolve_type_name(n, t.span);
                    }
                }
                self.walk_type(result);
            }
            TypeKind::Tuple(items) => {
                for i in items {
                    self.walk_type(i);
                }
            }
        }
    }

    fn resolve_type_name(&mut self, name: &str, span: Span) {
        if let Some(def) = self
            .res
            .defs
            .iter()
            .find(|d| d.name == name && matches!(d.kind, DefKind::Type | DefKind::Trait))
        {
            self.res.refs.insert(span, ResolvedName::TopLevel(def.id));
        } else {
            self.errors.push(Error {
                span,
                kind: ErrorKind::Unresolved {
                    name: name.to_string(),
                },
            });
        }
    }

    fn walk_method(&mut self, decl: &Decl) {
        if let DeclKind::Binding { value: Some(v), .. } = &decl.node {
            self.scope.push_frame();
            let _ = self.scope.push_local("self");
            self.walk_expr(v);
            self.scope.pop_frame();
        }
    }

    fn walk_expr(&mut self, e: &Expr) {
        match &e.node {
            ExprKind::Var(name) => self.resolve_var(name, e.span),
            ExprKind::IntLit(_) | ExprKind::FloatLit(_) | ExprKind::StringLit(_) => {}
            ExprKind::Ctor(name) => self.resolve_ctor(name, e.span),
            ExprKind::Paren(inner) => self.walk_expr(inner),
            ExprKind::BinOp { lhs, rhs, .. } => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            ExprKind::UnaryOp { expr, .. } => self.walk_expr(expr),
            ExprKind::List(items) => items.iter().for_each(|i| self.walk_expr(i)),
            ExprKind::Lambda { params, body } => {
                self.scope.push_frame();
                for p in params {
                    self.bind_pattern(p);
                }
                self.walk_expr(body);
                self.scope.pop_frame();
            }
            ExprKind::Construct { type_name, fields } => {
                self.resolve_type_or_ctor(type_name, e.span);
                for kw in fields {
                    self.walk_expr(&kw.value);
                }
            }
            ExprKind::Update { value, fields } => {
                self.walk_expr(value);
                for kw in fields {
                    self.walk_expr(&kw.value);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee);
                for arm in arms {
                    self.scope.push_frame();
                    self.bind_pattern(&arm.pattern);
                    self.walk_expr(&arm.body);
                    self.scope.pop_frame();
                }
            }
            ExprKind::Call { func, args } => {
                self.walk_expr(func);
                for a in args {
                    self.walk_expr(a);
                }
            }
            ExprKind::MethodCall { receiver, .. } => self.walk_expr(receiver),
            ExprKind::FieldAccess { receiver, .. } => self.walk_expr(receiver),
            ExprKind::Bang(inner) | ExprKind::Question(inner) => self.walk_expr(inner),
            ExprKind::Block(items) => self.walk_block(items),
        }
    }

    fn walk_block(&mut self, items: &[crate::ast::BlockItem]) {
        self.scope.push_frame();
        for item in items {
            match item {
                crate::ast::BlockItem::Expr(e) => self.walk_expr(e),
                crate::ast::BlockItem::Binding(decl) => {
                    if let DeclKind::Binding {
                        name,
                        value: Some(v),
                        ..
                    } = &decl.node
                    {
                        self.walk_expr(v);
                        if self.scope.push_local(name).is_err() {
                            self.errors.push(Error {
                                span: decl.span,
                                kind: ErrorKind::DuplicateLocal { name: name.clone() },
                            });
                        }
                    }
                }
            }
        }
        self.scope.pop_frame();
    }

    fn bind_pattern(&mut self, p: &Pattern) {
        match &p.node {
            PatternKind::Wildcard | PatternKind::Lit(_) => {}
            PatternKind::Var(name) => {
                if self.scope.push_local(name).is_err() {
                    self.errors.push(Error {
                        span: p.span,
                        kind: ErrorKind::DuplicateLocal { name: name.clone() },
                    });
                }
            }
            PatternKind::Ctor { name, args } => {
                self.resolve_ctor(name, p.span);
                for sub in args {
                    self.bind_pattern(sub);
                }
            }
            PatternKind::Tuple(items) | PatternKind::List(items) => {
                for sub in items {
                    self.bind_pattern(sub);
                }
            }
            PatternKind::Record { type_name, fields } => {
                self.resolve_type_or_ctor(type_name, p.span);
                for fp in fields {
                    self.bind_pattern(&fp.pattern);
                }
            }
        }
    }

    fn resolve_var(&mut self, name: &str, span: Span) {
        if let Some(id) = self.scope.lookup_local(name) {
            self.res.refs.insert(span, ResolvedName::Local(id));
            return;
        }
        if let Some(def) = self
            .res
            .defs
            .iter()
            .find(|d| d.name == name && matches!(d.kind, DefKind::Value))
        {
            self.res.refs.insert(span, ResolvedName::TopLevel(def.id));
            return;
        }
        self.errors.push(Error {
            span,
            kind: ErrorKind::Unresolved {
                name: name.to_string(),
            },
        });
    }

    fn resolve_ctor(&mut self, name: &str, span: Span) {
        if let Some(def) = self
            .res
            .defs
            .iter()
            .find(|d| d.name == name && matches!(d.kind, DefKind::Ctor { .. }))
        {
            self.res.refs.insert(span, ResolvedName::Ctor(def.id));
        } else {
            self.errors.push(Error {
                span,
                kind: ErrorKind::Unresolved {
                    name: name.to_string(),
                },
            });
        }
    }

    fn resolve_type_or_ctor(&mut self, name: &str, span: Span) {
        if let Some(def) = self.res.defs.iter().find(|d| d.name == name) {
            let resolved = match def.kind {
                DefKind::Ctor { .. } => ResolvedName::Ctor(def.id),
                _ => ResolvedName::TopLevel(def.id),
            };
            self.res.refs.insert(span, resolved);
        } else {
            self.errors.push(Error {
                span,
                kind: ErrorKind::Unresolved {
                    name: name.to_string(),
                },
            });
        }
    }
}

pub(super) fn walk_file(file: &File, res: &mut Resolution, errors: &mut Vec<Error>) {
    let mut w = Walker {
        res,
        errors,
        scope: ScopeStack::new(),
    };
    w.walk_file(file);
}
