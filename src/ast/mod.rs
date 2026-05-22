use crate::span::Spanned;

mod display;

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
