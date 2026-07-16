use crate::token::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub exports: Vec<String>,
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Module { alias: Option<String> },
    Symbol { name: String, alias: Option<String> },
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub module: Vec<String>,
    pub kind: ImportKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Int,
    Float,
    Number,
    String,
    Bool,
    List(Option<Box<TypeRef>>),
    None,
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Name(String, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Member(Box<Expr>, String, Span),
    Destructure(Vec<String>, Span),
}

#[derive(Debug, Clone)]
pub enum ClassMember {
    Field {
        name: String,
        value: Expr,
        private: bool,
        span: Span,
    },
    Method {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeRef>,
        body: Block,
        private: bool,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum AssignOp {
    Set,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone)]
pub enum LoopKind {
    Forever,
    While(Expr),
    For {
        name: String,
        iterable: Expr,
        step: Option<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign {
        target: AssignTarget,
        ty: Option<TypeRef>,
        value: Expr,
        constant: bool,
        op: AssignOp,
        span: Span,
    },
    Function {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeRef>,
        body: Block,
        span: Span,
    },
    Class {
        name: String,
        members: Vec<ClassMember>,
        span: Span,
    },
    If {
        condition: Expr,
        then_block: Block,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    Loop {
        kind: LoopKind,
        body: Block,
        span: Span,
    },
    Return(Option<Expr>, Span),
    Break(Span),
    Continue(Span),
    Expr(Expr),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Self::Assign { span, .. }
            | Self::Function { span, .. }
            | Self::Class { span, .. }
            | Self::If { span, .. }
            | Self::Loop { span, .. }
            | Self::Return(_, span)
            | Self::Break(span)
            | Self::Continue(span) => *span,
            Self::Expr(e) => e.span(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Negate,
    Positive,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Remainder,
    Power,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    In,
}

#[derive(Debug, Clone)]
pub enum Expr {
    None(Span),
    Bool(bool, Span),
    Int(i64, Span),
    Float(f64, Span),
    String(String, Span),
    List(Vec<Expr>, Span),
    Name(String, Span),
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MemberCall {
        object: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        name: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Slice {
        object: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::None(s)
            | Self::Bool(_, s)
            | Self::Int(_, s)
            | Self::Float(_, s)
            | Self::String(_, s)
            | Self::List(_, s)
            | Self::Name(_, s) => *s,
            Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Call { span, .. }
            | Self::MemberCall { span, .. }
            | Self::Member { span, .. }
            | Self::Index { span, .. }
            | Self::Slice { span, .. }
            | Self::Range { span, .. } => *span,
        }
    }
}
