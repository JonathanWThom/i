use crate::ast::*;

pub fn pretty(file: &File) -> String {
    let mut p = Printer::new();
    p.write_file(file);
    p.out
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
        self.indent += 4;
    }

    fn indent_out(&mut self) {
        self.indent = self.indent.saturating_sub(4);
    }

    fn write_file(&mut self, file: &File) {
        if let Some(m) = &file.module {
            self.write_module_header(m);
            self.out.push('\n');
        }
        for d in &file.decls {
            self.write_decl(d);
            self.out.push('\n');
        }
    }

    fn write_module_header(&mut self, m: &ModuleHeader) {
        self.push("module ");
        self.push(&m.name.join("."));
        self.indent_in();
        self.nl();
        self.push("expose ");
        for (i, e) in m.exposes.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.write_expose(e);
        }
        self.indent_out();
    }

    fn write_expose(&mut self, e: &Expose) {
        match e {
            Expose::Value(n) => self.push(n),
            Expose::Type {
                name,
                with_constructors,
            } => {
                self.push(name);
                if *with_constructors {
                    self.push("(..)");
                }
            }
        }
    }

    fn write_decl(&mut self, d: &Decl) {
        match &d.node {
            DeclKind::Binding { name, ty, value } => {
                self.write_binding(name, ty.as_ref(), value.as_ref())
            }
            DeclKind::TypeDecl { name, params, body } => self.write_type_decl(name, params, body),
            DeclKind::TraitDecl {
                name,
                type_var,
                methods,
            } => self.write_trait_decl(name, type_var, methods),
            DeclKind::ImplDecl {
                trait_name,
                target,
                methods,
            } => self.write_impl_decl(trait_name, target, methods),
            DeclKind::Use { path, kind } => self.write_use(path, kind),
        }
    }

    fn write_binding(&mut self, name: &str, ty: Option<&Type>, value: Option<&Expr>) {
        self.push(name);
        if let Some(t) = ty {
            self.push(" : ");
            self.write_type(t);
        }
        if let Some(v) = value {
            self.push(" =");
            self.write_value_body(v);
        }
    }

    fn write_value_body(&mut self, v: &Expr) {
        if let ExprKind::Block(items) = &v.node {
            self.indent_in();
            for item in items {
                self.nl();
                self.write_block_item(item);
            }
            self.indent_out();
        } else {
            self.push(" ");
            self.write_expr(v);
        }
    }

    fn write_block_item(&mut self, item: &BlockItem) {
        match item {
            BlockItem::Binding(d) => self.write_decl(d),
            BlockItem::Expr(e) => self.write_expr(e),
        }
    }

    fn write_type_decl(&mut self, name: &str, params: &[String], body: &TypeBody) {
        self.push("type ");
        self.push(name);
        for p in params {
            self.push(" ");
            self.push(p);
        }
        match body {
            TypeBody::Newtype(t) => {
                self.push(" = ");
                self.write_type(t);
            }
            TypeBody::Block(members) => {
                self.indent_in();
                for m in members {
                    self.nl();
                    self.write_type_member(m);
                }
                self.indent_out();
            }
        }
    }

    fn write_type_member(&mut self, m: &TypeMember) {
        match m {
            TypeMember::Field { name, ty } => {
                self.push(name);
                self.push(" : ");
                self.write_type(ty);
            }
            TypeMember::Method(d) => self.write_decl(d),
            TypeMember::Variant { name, body } => {
                self.push(name);
                match body {
                    VariantBody::Bare => {}
                    VariantBody::Single(t) => {
                        self.push(" : ");
                        self.write_type(t);
                    }
                    VariantBody::Fields(fields) => {
                        self.indent_in();
                        for f in fields {
                            self.nl();
                            self.write_type_member(f);
                        }
                        self.indent_out();
                    }
                }
            }
        }
    }

    fn write_trait_decl(&mut self, name: &str, type_var: &str, methods: &[Decl]) {
        self.push("trait ");
        self.push(name);
        self.push(" ");
        self.push(type_var);
        self.indent_in();
        for m in methods {
            self.nl();
            self.write_decl(m);
        }
        self.indent_out();
    }

    fn write_impl_decl(&mut self, trait_name: &str, target: &Type, methods: &[Decl]) {
        self.push("impl ");
        self.push(trait_name);
        self.push(" ");
        self.write_type(target);
        self.indent_in();
        for m in methods {
            self.nl();
            self.write_decl(m);
        }
        self.indent_out();
    }

    fn write_use(&mut self, path: &[String], kind: &UseKind) {
        self.push("use ");
        for (i, p) in path.iter().enumerate() {
            if i > 0 {
                self.push(".");
            }
            self.push(p);
        }
        match kind {
            UseKind::Whole => {}
            UseKind::Alias(a) => {
                self.push(" as ");
                self.push(a);
            }
            UseKind::Cherry(names) => {
                self.push(" (");
                for (i, n) in names.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(n);
                }
                self.push(")");
            }
        }
    }

    fn write_expr(&mut self, e: &Expr) {
        match &e.node {
            ExprKind::IntLit(n) => self.push(&n.to_string()),
            ExprKind::FloatLit(f) => self.push(&format_float(*f)),
            ExprKind::StringLit(s) => {
                self.push("\"");
                self.push(s);
                self.push("\"");
            }
            ExprKind::Var(n) => self.push(n),
            ExprKind::Ctor(n) => self.push(n),
            ExprKind::List(items) => {
                self.push("[");
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.write_expr(it);
                }
                self.push("]");
            }
            ExprKind::Paren(inner) => {
                self.push("(");
                self.write_expr(inner);
                self.push(")");
            }
            ExprKind::BinOp { op, lhs, rhs } => {
                self.write_expr(lhs);
                self.push(" ");
                self.push(binop_text(op));
                self.push(" ");
                self.write_expr(rhs);
            }
            ExprKind::UnaryOp { op, expr } => {
                self.push(unaryop_text(op));
                if matches!(op, UnaryOp::Not) {
                    self.push(" ");
                }
                self.write_expr(expr);
            }
            ExprKind::Lambda { params, body } => {
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(" ");
                    }
                    self.write_pattern(p);
                }
                self.push(" ->");
                self.write_value_body(body);
            }
            ExprKind::Call { func, args } => {
                self.write_expr(func);
                for (i, a) in args.iter().enumerate() {
                    if i == 0 {
                        self.push(" ");
                    } else {
                        self.push(", ");
                    }
                    self.write_expr(a);
                }
            }
            ExprKind::MethodCall { receiver, method } => {
                self.write_expr(receiver);
                self.push(".");
                self.push(method);
            }
            ExprKind::FieldAccess { receiver, field } => {
                self.write_expr(receiver);
                self.push(".");
                self.push(field);
            }
            ExprKind::Construct { type_name, fields } => {
                self.push(type_name);
                self.write_kwargs(fields);
            }
            ExprKind::Update { value, fields } => {
                self.write_expr(value);
                self.write_kwargs(fields);
            }
            ExprKind::Bang(inner) => {
                self.write_expr(inner);
                self.push("!");
            }
            ExprKind::Question(inner) => {
                self.write_expr(inner);
                self.push("?");
            }
            ExprKind::Match { scrutinee, arms } => {
                self.write_expr(scrutinee);
                self.push(" match");
                self.indent_in();
                for arm in arms {
                    self.nl();
                    self.write_pattern(&arm.pattern);
                    self.push(" -> ");
                    self.write_expr(&arm.body);
                }
                self.indent_out();
            }
            ExprKind::Block(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.nl();
                    }
                    self.write_block_item(item);
                }
            }
        }
    }

    fn write_kwargs(&mut self, fields: &[KwArg]) {
        self.push("(");
        for (i, f) in fields.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.push(&f.name);
            self.push(" = ");
            self.write_expr(&f.value);
        }
        self.push(")");
    }

    fn write_pattern(&mut self, p: &Pattern) {
        match &p.node {
            PatternKind::Wildcard => self.push("_"),
            PatternKind::Var(n) => self.push(n),
            PatternKind::Lit(l) => match l {
                LitPat::Int(n) => self.push(&n.to_string()),
                LitPat::Float(f) => self.push(&format_float(*f)),
                LitPat::Str(s) => {
                    self.push("\"");
                    self.push(s);
                    self.push("\"");
                }
            },
            PatternKind::Ctor { name, args } => {
                self.push(name);
                for (i, a) in args.iter().enumerate() {
                    if i == 0 {
                        self.push(" ");
                    } else {
                        self.push(", ");
                    }
                    self.write_pattern(a);
                }
            }
            PatternKind::Record { type_name, fields } => {
                self.push(type_name);
                self.push("(");
                for (i, f) in fields.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(&f.field);
                    self.push(" = ");
                    self.write_pattern(&f.pattern);
                }
                self.push(")");
            }
            PatternKind::Tuple(items) => {
                self.push("(");
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.write_pattern(it);
                }
                self.push(")");
            }
            PatternKind::List(items) => {
                self.push("[");
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.write_pattern(it);
                }
                self.push("]");
            }
        }
    }

    fn write_type(&mut self, t: &Type) {
        match &t.node {
            TypeKind::Var(n) => self.push(n),
            TypeKind::Named { name, args } => {
                self.push(name);
                for a in args {
                    self.push(" ");
                    self.write_type(a);
                }
            }
            TypeKind::Function {
                params,
                effect,
                result,
            } => {
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.write_type(p);
                }
                if let Some(eff) = effect {
                    self.push(" ! ");
                    self.write_effect(eff);
                }
                self.push(" -> ");
                self.write_type(result);
            }
            TypeKind::Tuple(items) => {
                self.push("(");
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.write_type(it);
                }
                self.push(")");
            }
        }
    }

    fn write_effect(&mut self, e: &EffectRow) {
        match e {
            EffectRow::Empty => self.push("()"),
            EffectRow::Named(names) => {
                for (i, n) in names.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(n);
                }
            }
        }
    }
}

fn binop_text(op: &BinOp) -> &'static str {
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

fn unaryop_text(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "not",
    }
}

fn format_float(f: f64) -> String {
    if f.fract() == 0.0 && f.is_finite() {
        format!("{:.1}", f)
    } else {
        format!("{}", f)
    }
}
