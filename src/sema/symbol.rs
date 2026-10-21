use crate::syntax::token::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    /// Read-only immutable binding (can be copied directly when captured as an upvalue)
    Constant,
    /// Mutable variable (if re-assigned; if captured into closure, requires box/cell)
    Mutable,
    /// Captured from enclosing scope
    Upvalue {
        is_const: bool,
        parent_index: usize,
        is_direct_local: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedVar {
    /// Local variable in the current activation frame
    Local {
        slot: usize,
        kind: VarKind,
        name: String,
        span: Span,
    },
    /// Upvalue captured from an outer function scope
    Upvalue {
        upvalue_index: usize,
        kind: VarKind,
        name: String,
        span: Span,
    },
    /// Top-level global function / constant / protocol
    Global {
        name: String,
        span: Span,
    },
    /// Namespace / Module container (e.g. `std`, `math`, `std.io`)
    Namespace {
        path: Vec<String>,
        span: Span,
    },
    /// Fully qualified symbol (e.g. `std.io.println`, `math.sin`)
    QualifiedGlobal {
        path: Vec<String>,
        span: Span,
    },
    /// Enum variant tag (e.g. `:Circle`, `:Rect`, `:Ok`, `:Err`)
    EnumVariant {
        enum_name: Option<String>,
        variant_name: String,
        span: Span,
    },
}

impl ResolvedVar {
    pub fn span(&self) -> Span {
        match self {
            ResolvedVar::Local { span, .. } => *span,
            ResolvedVar::Upvalue { span, .. } => *span,
            ResolvedVar::Global { span, .. } => *span,
            ResolvedVar::Namespace { span, .. } => *span,
            ResolvedVar::QualifiedGlobal { span, .. } => *span,
            ResolvedVar::EnumVariant { span, .. } => *span,
        }
    }
}
