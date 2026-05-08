use crate::span::Spanned;
use std::fmt;

pub type Expr = Spanned<ExprKind>;
pub type Type = Spanned<TypeKind>;
pub type Pattern = Spanned<PatternKind>;
pub type Decl = Spanned<DeclKind>;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub module: Option<ModuleHeader>,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleHeader {
    pub name: String,
    pub exposes: Vec<Expose>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expose {
    Value(String),
    Type {
        name: String,
        with_constructors: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    Var(String),
    Ctor(String),
    List(Vec<Expr>),
    Paren(Box<Expr>),

    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    Lambda {
        params: Vec<Pattern>,
        body: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
    },
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
    },
    Construct {
        type_name: String,
        fields: Vec<KwArg>,
    },
    Update {
        value: Box<Expr>,
        fields: Vec<KwArg>,
    },

    Bang(Box<Expr>),
    Question(Box<Expr>),

    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Block(Vec<BlockItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Concat,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KwArg {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    Binding(Decl),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    Wildcard,
    Var(String),
    Lit(LitPat),
    Ctor {
        name: String,
        args: Vec<Pattern>,
    },
    Record {
        type_name: String,
        fields: Vec<FieldPat>,
    },
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitPat {
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPat {
    pub field: String,
    pub pattern: Pattern,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Var(String),
    Named {
        name: String,
        args: Vec<Type>,
    },
    Function {
        params: Vec<Type>,
        effect: Option<EffectRow>,
        result: Box<Type>,
    },
    Tuple(Vec<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectRow {
    Empty,
    Named(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    Binding {
        name: String,
        ty: Option<Type>,
        value: Option<Expr>,
    },
    TypeDecl {
        name: String,
        params: Vec<String>,
        body: TypeBody,
    },
    TraitDecl {
        name: String,
        type_var: String,
        methods: Vec<Decl>,
    },
    ImplDecl {
        trait_name: String,
        target: Type,
        methods: Vec<Decl>,
    },
    Use {
        path: Vec<String>,
        kind: UseKind,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBody {
    Newtype(Type),
    Block(Vec<TypeMember>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMember {
    Field { name: String, ty: Type },
    Method(Decl),
    Variant { name: String, body: VariantBody },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantBody {
    Bare,
    Single(Type),
    Fields(Vec<TypeMember>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UseKind {
    Whole,
    Cherry(Vec<String>),
    Alias(String),
}

impl File {
    pub fn node_eq(&self, other: &File) -> bool {
        format!("{}", self) == format!("{}", other)
    }
}

struct Printer {
    out: String,
    indent: usize,
}

impl Printer {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
        }
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn nl(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push(' ');
        }
    }

    fn indent_in(&mut self) {
        self.indent += 2;
    }

    fn indent_out(&mut self) {
        self.indent = self.indent.saturating_sub(2);
    }

    fn write_file(&mut self, file: &File) {
        self.push("(file");
        if let Some(m) = &file.module {
            self.indent_in();
            self.nl();
            self.push("(module ");
            self.push(&m.name);
            for e in &m.exposes {
                self.push(" ");
                match e {
                    Expose::Value(n) => {
                        self.push("(expose-val ");
                        self.push(n);
                        self.push(")");
                    }
                    Expose::Type {
                        name,
                        with_constructors,
                    } => {
                        self.push("(expose-type ");
                        self.push(name);
                        if *with_constructors {
                            self.push(" *");
                        }
                        self.push(")");
                    }
                }
            }
            self.push(")");
            self.indent_out();
        }
        if !file.decls.is_empty() {
            self.indent_in();
            for decl in &file.decls {
                self.nl();
                self.write_decl(decl);
            }
            self.indent_out();
        }
        self.push(")");
    }

    fn write_decl(&mut self, decl: &Decl) {
        match &decl.node {
            DeclKind::Binding { name, ty, value } => match value {
                None => {
                    self.push("(sig ");
                    self.push(name);
                    if let Some(t) = ty {
                        self.push(" ");
                        self.write_type(t);
                    }
                    self.push(")");
                }
                Some(v) => {
                    self.push("(let ");
                    self.push(name);
                    if let Some(t) = ty {
                        self.push(" : ");
                        self.write_type(t);
                    }
                    self.push(" ");
                    self.write_expr(v);
                    self.push(")");
                }
            },
            DeclKind::TypeDecl { name, params, body } => {
                self.push("(type ");
                self.push(name);
                for p in params {
                    self.push(" ");
                    self.push(p);
                }
                match body {
                    TypeBody::Newtype(t) => {
                        self.push(" ");
                        self.write_type(t);
                        self.push(")");
                    }
                    TypeBody::Block(members) => {
                        self.indent_in();
                        for m in members {
                            self.nl();
                            self.write_type_member(m);
                        }
                        self.indent_out();
                        self.push(")");
                    }
                }
            }
            DeclKind::TraitDecl {
                name,
                type_var,
                methods,
            } => {
                self.push("(trait ");
                self.push(name);
                self.push(" ");
                self.push(type_var);
                self.indent_in();
                for m in methods {
                    self.nl();
                    self.write_decl(m);
                }
                self.indent_out();
                self.push(")");
            }
            DeclKind::ImplDecl {
                trait_name,
                target,
                methods,
            } => {
                self.push("(impl ");
                self.push(trait_name);
                self.push(" ");
                self.write_type(target);
                self.indent_in();
                for m in methods {
                    self.nl();
                    self.write_decl(m);
                }
                self.indent_out();
                self.push(")");
            }
            DeclKind::Use { path, kind } => {
                self.push("(use ");
                self.push(&path.join("."));
                match kind {
                    UseKind::Whole => {}
                    UseKind::Cherry(names) => {
                        self.push(" (");
                        self.push(&names.join(" "));
                        self.push(")");
                    }
                    UseKind::Alias(a) => {
                        self.push(" as ");
                        self.push(a);
                    }
                }
                self.push(")");
            }
        }
    }

    fn write_type_member(&mut self, m: &TypeMember) {
        match m {
            TypeMember::Field { name, ty } => {
                self.push("(field ");
                self.push(name);
                self.push(" ");
                self.write_type(ty);
                self.push(")");
            }
            TypeMember::Method(d) => self.write_decl(d),
            TypeMember::Variant { name, body } => {
                self.push("(variant ");
                self.push(name);
                match body {
                    VariantBody::Bare => {}
                    VariantBody::Single(t) => {
                        self.push(" ");
                        self.write_type(t);
                    }
                    VariantBody::Fields(members) => {
                        self.indent_in();
                        for m in members {
                            self.nl();
                            self.write_type_member(m);
                        }
                        self.indent_out();
                    }
                }
                self.push(")");
            }
        }
    }

    fn write_expr(&mut self, expr: &Expr) {
        self.write_expr_kind(&expr.node);
    }

    fn write_expr_kind(&mut self, kind: &ExprKind) {
        match kind {
            ExprKind::IntLit(n) => {
                self.push("(int ");
                self.push(&n.to_string());
                self.push(")");
            }
            ExprKind::FloatLit(f) => {
                self.push("(float ");
                self.push(&f.to_string());
                self.push(")");
            }
            ExprKind::StringLit(s) => {
                self.push("(str \"");
                self.push(s);
                self.push("\")");
            }
            ExprKind::Var(n) => {
                self.push("(var ");
                self.push(n);
                self.push(")");
            }
            ExprKind::Ctor(n) => {
                self.push("(ctor ");
                self.push(n);
                self.push(")");
            }
            ExprKind::List(items) => {
                self.push("(list");
                for i in items {
                    self.push(" ");
                    self.write_expr(i);
                }
                self.push(")");
            }
            ExprKind::Paren(inner) => {
                self.push("(paren ");
                self.write_expr(inner);
                self.push(")");
            }
            ExprKind::BinOp { op, lhs, rhs } => {
                self.push("(");
                self.push(binop_name(op));
                self.push(" ");
                self.write_expr(lhs);
                self.push(" ");
                self.write_expr(rhs);
                self.push(")");
            }
            ExprKind::UnaryOp { op, expr } => {
                self.push("(");
                self.push(unaryop_name(op));
                self.push(" ");
                self.write_expr(expr);
                self.push(")");
            }
            ExprKind::Lambda { params, body } => {
                self.push("(lambda (");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(" ");
                    }
                    self.write_pattern(p);
                }
                self.push(") ");
                self.write_expr(body);
                self.push(")");
            }
            ExprKind::Call { func, args } => {
                self.push("(call ");
                self.write_expr(func);
                for a in args {
                    self.push(" ");
                    self.write_expr(a);
                }
                self.push(")");
            }
            ExprKind::MethodCall { receiver, method } => {
                self.push("(method ");
                self.write_expr(receiver);
                self.push(" ");
                self.push(method);
                self.push(")");
            }
            ExprKind::FieldAccess { receiver, field } => {
                self.push("(. ");
                self.write_expr(receiver);
                self.push(" ");
                self.push(field);
                self.push(")");
            }
            ExprKind::Construct { type_name, fields } => {
                self.push("(construct ");
                self.push(type_name);
                for kw in fields {
                    self.push(" ");
                    self.write_kwarg(kw);
                }
                self.push(")");
            }
            ExprKind::Update { value, fields } => {
                self.push("(update ");
                self.write_expr(value);
                for kw in fields {
                    self.push(" ");
                    self.write_kwarg(kw);
                }
                self.push(")");
            }
            ExprKind::Bang(e) => {
                self.push("(! ");
                self.write_expr(e);
                self.push(")");
            }
            ExprKind::Question(e) => {
                self.push("(? ");
                self.write_expr(e);
                self.push(")");
            }
            ExprKind::Match { scrutinee, arms } => {
                self.push("(match ");
                self.write_expr(scrutinee);
                self.indent_in();
                for arm in arms {
                    self.nl();
                    self.push("(arm ");
                    self.write_pattern(&arm.pattern);
                    self.push(" ");
                    self.write_expr(&arm.body);
                    self.push(")");
                }
                self.indent_out();
                self.push(")");
            }
            ExprKind::Block(items) => {
                self.push("(block");
                self.indent_in();
                for item in items {
                    self.nl();
                    match item {
                        BlockItem::Binding(d) => self.write_decl(d),
                        BlockItem::Expr(e) => self.write_expr(e),
                    }
                }
                self.indent_out();
                self.push(")");
            }
        }
    }

    fn write_kwarg(&mut self, kw: &KwArg) {
        self.push("(= ");
        self.push(&kw.name);
        self.push(" ");
        self.write_expr(&kw.value);
        self.push(")");
    }

    fn write_pattern(&mut self, pat: &Pattern) {
        match &pat.node {
            PatternKind::Wildcard => self.push("_"),
            PatternKind::Var(n) => {
                self.push("(var ");
                self.push(n);
                self.push(")");
            }
            PatternKind::Lit(l) => match l {
                LitPat::Int(n) => {
                    self.push("(int ");
                    self.push(&n.to_string());
                    self.push(")");
                }
                LitPat::Float(f) => {
                    self.push("(float ");
                    self.push(&f.to_string());
                    self.push(")");
                }
                LitPat::Str(s) => {
                    self.push("(str \"");
                    self.push(s);
                    self.push("\")");
                }
            },
            PatternKind::Ctor { name, args } => {
                self.push("(");
                self.push(name);
                for a in args {
                    self.push(" ");
                    self.write_pattern(a);
                }
                self.push(")");
            }
            PatternKind::Record { type_name, fields } => {
                self.push("(record ");
                self.push(type_name);
                for fp in fields {
                    self.push(" (= ");
                    self.push(&fp.field);
                    self.push(" ");
                    self.write_pattern(&fp.pattern);
                    self.push(")");
                }
                self.push(")");
            }
            PatternKind::Tuple(pats) => {
                self.push("(tuple");
                for p in pats {
                    self.push(" ");
                    self.write_pattern(p);
                }
                self.push(")");
            }
            PatternKind::List(pats) => {
                self.push("(list-pat");
                for p in pats {
                    self.push(" ");
                    self.write_pattern(p);
                }
                self.push(")");
            }
        }
    }

    fn write_type(&mut self, ty: &Type) {
        match &ty.node {
            TypeKind::Var(n) => self.push(n),
            TypeKind::Named { name, args } => {
                if args.is_empty() {
                    self.push(name);
                } else {
                    self.push("(");
                    self.push(name);
                    for a in args {
                        self.push(" ");
                        self.write_type(a);
                    }
                    self.push(")");
                }
            }
            TypeKind::Function {
                params,
                effect,
                result,
            } => {
                self.push("(-> (");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(" ");
                    }
                    self.write_type(p);
                }
                self.push(")");
                if let Some(e) = effect {
                    self.push(" ");
                    self.write_effect(e);
                }
                self.push(" ");
                self.write_type(result);
                self.push(")");
            }
            TypeKind::Tuple(types) => {
                self.push("(tuple");
                for t in types {
                    self.push(" ");
                    self.write_type(t);
                }
                self.push(")");
            }
        }
    }

    fn write_effect(&mut self, e: &EffectRow) {
        match e {
            EffectRow::Empty => self.push("(! ())"),
            EffectRow::Named(names) => {
                self.push("(!");
                for n in names {
                    self.push(" ");
                    self.push(n);
                }
                self.push(")");
            }
        }
    }
}

fn binop_name(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Pow => "^",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Concat => "++",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::Xor => "xor",
    }
}

fn unaryop_name(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "neg",
        UnaryOp::Not => "not",
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut p = Printer::new();
        p.write_file(self);
        f.write_str(&p.out)
    }
}

impl fmt::Display for ExprKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut p = Printer::new();
        p.write_expr_kind(self);
        f.write_str(&p.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn sp<T>(node: T) -> Spanned<T> {
        Spanned {
            span: Span::new(0, 0),
            node,
        }
    }

    #[test]
    fn display_simple_let() {
        let file = File {
            module: None,
            decls: vec![sp(DeclKind::Binding {
                name: "x".into(),
                ty: None,
                value: Some(sp(ExprKind::IntLit(1))),
            })],
        };
        let out = format!("{}", file);
        assert_eq!(out.trim(), "(file\n  (let x (int 1)))");
    }
}
