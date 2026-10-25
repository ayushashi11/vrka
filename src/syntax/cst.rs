use std::path::PathBuf;

use super::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum LitVal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,
    Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnn>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub type_ann: TypeAnn,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantDef {
    pub name: String,
    pub payload: Vec<TypeAnn>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnSig {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub ret_type: Option<TypeAnn>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnn {
    Named {
        name: String,
        args: Vec<TypeAnn>,
        span: Span,
    },
    Tuple(Vec<TypeAnn>, Span),
    Fn {
        params: Vec<TypeAnn>,
        ret: Box<TypeAnn>,
        span: Span,
    },
}

impl TypeAnn {
    pub fn span(&self) -> Span {
        match self {
            TypeAnn::Named { span, .. } => *span,
            TypeAnn::Tuple(_, span) => *span,
            TypeAnn::Fn { span, .. } => *span,
        }
    }
    pub fn get_compiled_name(&self) -> String {
        match self {
            TypeAnn::Named { name, args, .. } => {
                if args.is_empty() {
                    name.clone()
                } else {
                    let args_str = args
                        .iter()
                        .map(|arg| arg.get_compiled_name())
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("{}<{}>", name, args_str)
                }
            }
            TypeAnn::Tuple(elements, _) => {
                let elements_str = elements
                    .iter()
                    .map(|elem| elem.get_compiled_name())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({})", elements_str)
            }
            TypeAnn::Fn { params, ret, .. } => {
                let params_str = params
                    .iter()
                    .map(|param| param.get_compiled_name())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("Fn({}):{}", params_str, ret.get_compiled_name())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternNode {
    Wildcard(Span),
    Literal(LitVal, Span),
    Bind(String, Span),
    Tuple(Vec<PatternNode>, Span),
    Struct {
        name: Option<String>,
        fields: Vec<StructPatternNode>,
        span: Span,
    },
    EnumVariant {
        name: String,
        args: Vec<PatternNode>,
        span: Span,
    },
    Array {
        segments: Vec<PatternSegment>,
        span: Span,
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        span: Span,
    },
}

impl PatternNode {
    pub fn span(&self) -> Span {
        match self {
            PatternNode::Wildcard(s) => *s,
            PatternNode::Literal(_, s) => *s,
            PatternNode::Bind(_, s) => *s,
            PatternNode::Tuple(_, s) => *s,
            PatternNode::EnumVariant { span, .. } => *span,
            PatternNode::Struct { span, .. } => *span,
            PatternNode::Array { span, .. } => *span,
            PatternNode::Range { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructPatternNode {
    KeyValue {
        key: String,
        value: Box<PatternNode>,
        span: Span,
    },
    Shorthand {
        key: String,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternSegment {
    Element(PatternNode),
    BindList(String, Span),
    WildcardList(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: PatternNode,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail_expr: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(LitVal, Span),
    InterpolatedString(Vec<StringPart>, Span),
    Ident(String, Span),
    Atom(String, Span),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    ArrayLit(Vec<Expr>, Span),
    TupleLit(Vec<Expr>, Span),
    RecordLit(Vec<(String, Expr)>, Span),
    Block(Block),
    If {
        cond: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Spawn {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Send {
        receiver: Box<Expr>,
        message: Box<Expr>,
        span: Span,
    },
    Receive(Span),
    Lambda {
        params: Vec<Param>,
        ret_type: Option<TypeAnn>,
        body: Block,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(_, s) => *s,
            Expr::InterpolatedString(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::Atom(_, s) => *s,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::ArrayLit(_, s) => *s,
            Expr::TupleLit(_, s) => *s,
            Expr::RecordLit(_, s) => *s,
            Expr::Block(b) => b.span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Spawn { span, .. } => *span,
            Expr::Send { span, .. } => *span,
            Expr::Receive(s) => *s,
            Expr::Lambda { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        type_ann: Option<TypeAnn>,
        init: Expr,
        span: Span,
    },
    Const {
        name: String,
        type_ann: Option<TypeAnn>,
        init: Expr,
        span: Span,
    },
    Assign {
        target: Expr,
        val: Expr,
        span: Span,
    },
    AugAssign {
        target: Expr,
        op: BinOp,
        val: Expr,
        span: Span,
    },
    While {
        cond: Expr,
        body: Block,
        span: Span,
    },
    Loop {
        body: Block,
        span: Span,
    },
    Return {
        val: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Expr(Expr),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Const { span, .. } => *span,
            Stmt::Assign { span, .. } => *span,
            Stmt::AugAssign { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::Loop { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::Break(span) => *span,
            Stmt::Continue(span) => *span,
            Stmt::Expr(e) => e.span(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportTarget {
    /// import std.io, import std.net
    Dotted(Vec<String>),
    /// import "path/to/file.mesh"
    Path(String),
    /// import module_name
    Ident(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDef {
    pub target: ImportTarget,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Fn(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    Protocol(ProtocolDef),
    Impl(ImplDef),
    Import(ImportDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub ret_type: Option<TypeAnn>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<VariantDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub properties: Vec<FieldDef>,
    pub methods: Vec<FnSig>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    pub target_type: TypeAnn,
    pub for_protocol: Option<TypeAnn>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
    pub path: PathBuf,
    pub span: Span,
}
