use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVarId(pub usize);

impl fmt::Display for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowVarId(pub usize);

impl fmt::Display for RowVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ρ{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // ── Primitives ──────────────────────────────────────────────────────────
    Int,
    Float,
    Bool,
    String,
    Unit,
    Nil,
    Never,

    // ── Specialized Numeric Arrays & Ranges (Resolved during Type Resolution) ─
    IntArray,   // Unboxed i64 vector
    FloatArray, // Unboxed f64 vector
    IntRange {
        inclusive: bool,
    },
    FloatRange {
        inclusive: bool,
    },

    // ── Polymorphic & Compound Types ────────────────────────────────────────
    Array(Box<Type>), // Dynamic / boxed element array
    Tuple(Vec<Type>),

    // ── Row Polymorphic Record for Duck Typing ──────────────────────────────
    // An open record allows any object with at least these fields to unify!
    Record {
        fields: HashMap<String, Type>,
        row: Option<RowVarId>, // None = closed nominal struct, Some(r) = open duck-typed record
    },

    // ── Row Polymorphic Variant for Enum Tags (:Circle, :Rect) ──────────────
    Variant {
        cases: HashMap<String, Vec<Type>>,
        row: Option<RowVarId>, // None = closed enum, Some(r) = open extensible variant
    },

    // ── Protocol / Interface Capability ────────────────────────────────────
    Protocol {
        name: String,
        properties: HashMap<String, Type>,
        methods: HashMap<String, (Vec<Type>, Box<Type>)>,
    },

    // ── Function / Closure ──────────────────────────────────────────────────
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
    },

    // ── Type Inference Variable ─────────────────────────────────────────────
    Var(TypeVarId),
}

impl Type {
    pub fn is_int(&self) -> bool {
        matches!(self, Type::Int)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float)
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Unit => write!(f, "Unit"),
            Type::Nil => write!(f, "Nil"),
            Type::Never => write!(f, "Never"),
            Type::IntArray => write!(f, "IntArray"),
            Type::FloatArray => write!(f, "FloatArray"),
            Type::IntRange { inclusive } => {
                write!(f, "IntRange({})", if *inclusive { "..=" } else { ".." })
            }
            Type::FloatRange { inclusive } => {
                write!(f, "FloatRange({})", if *inclusive { "..=" } else { ".." })
            }
            Type::Array(inner) => write!(f, "Array[{}]", inner),
            Type::Tuple(items) => {
                write!(f, "(")?;
                for (i, t) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Record { fields, row } => {
                write!(f, "{{ ")?;
                let mut first = true;
                for (k, v) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}: {}", k, v)?;
                }
                if let Some(r) = row {
                    if !first {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", r)?;
                }
                write!(f, " }}")
            }
            Type::Variant { cases, row } => {
                write!(f, "[ ")?;
                let mut first = true;
                for (tag, args) in cases {
                    if !first {
                        write!(f, " | ")?;
                    }
                    first = false;
                    write!(f, ":{}", tag)?;
                    if !args.is_empty() {
                        write!(f, "(")?;
                        for (i, a) in args.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", a)?;
                        }
                        write!(f, ")")?;
                    }
                }
                if let Some(r) = row {
                    if !first {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", r)?;
                }
                write!(f, " ]")
            }
            Type::Protocol { name, .. } => write!(f, "Protocol({})", name),
            Type::Fn { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Var(v) => write!(f, "{}", v),
        }
    }
}
