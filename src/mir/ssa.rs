use crate::syntax::cst::{BinOp, LitVal, PatternNode, TypeAnn, UnaryOp};
use crate::syntax::token::Span;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct ValueData {
    pub id: ValueId,
    pub name: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Phi {
    pub dst: ValueId,
    pub incoming: Vec<(BlockId, ValueId)>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPartValue {
    Literal(String),
    Value(ValueId),
}

#[derive(Debug, Clone)]
pub enum SsaOp {
    Const(LitVal),
    BuildString(Vec<StringPartValue>),
    Binary(BinOp, ValueId, ValueId),
    Unary(UnaryOp, ValueId),
    Call {
        callee: ValueId,
        args: Vec<ValueId>,
    },
    FieldAccess {
        base: ValueId,
        field: String,
    },
    Index {
        base: ValueId,
        index: ValueId,
    },
    StoreField {
        base: ValueId,
        field: String,
        val: ValueId,
    },
    StoreIndex {
        base: ValueId,
        index: ValueId,
        val: ValueId,
    },
    ArrayLit(Vec<ValueId>),
    TupleLit(Vec<ValueId>),
    NewObject {
        name: String,
        prototype: String,
        fields: Vec<(String, TypeAnn)>,
        args: Vec<ValueId>,
    },
    Spawn {
        callee: ValueId,
        args: Vec<ValueId>,
    },
    Send {
        receiver: ValueId,
        message: ValueId,
    },
    Receive,
    This,
    GlobalRef(String),
    QualifiedGlobal(Vec<String>),
    Atom(String),
    Upvalue(usize),
    StoreUpvalue {
        index: usize,
        val: ValueId,
    },
    MakeEnv(Vec<ValueId>),
    MakeClosure {
        func: String,
        env: Option<ValueId>,
    },
    FormatValue {
        val: ValueId,
        spec: Option<ValueId>,
    },
}

#[derive(Debug, Clone)]
pub struct SsaInst {
    pub dst: Option<ValueId>,
    pub op: SsaOp,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SsaMatchArm {
    pub pattern: PatternNode,
    pub target_bb: BlockId,
    pub bindings: Vec<(String, ValueId)>,
}

#[derive(Debug, Clone)]
pub enum SsaTerminator {
    Return(Option<ValueId>, Span),
    Goto(BlockId, Span),
    Branch {
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
        span: Span,
    },
    Match {
        target: ValueId,
        arms: Vec<SsaMatchArm>,
        default_bb: BlockId,
        span: Span,
    },
    Unreachable(Span),
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub label: String,
    pub phis: Vec<Phi>,
    pub instructions: Vec<SsaInst>,
    pub terminator: SsaTerminator,
    pub predecessors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId, label: String) -> Self {
        Self {
            id,
            label,
            phis: Vec::new(),
            instructions: Vec::new(),
            terminator: SsaTerminator::Unreachable(Span::default()),
            predecessors: Vec::new(),
        }
    }

    pub fn is_terminated(&self) -> bool {
        !matches!(self.terminator, SsaTerminator::Unreachable(_))
    }
}

#[derive(Debug, Clone)]
pub struct SsaFunction {
    pub name: String,
    pub params: Vec<(String, ValueId, Option<TypeAnn>)>,
    pub ret_type: Option<TypeAnn>,
    pub values: Vec<ValueData>,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
    pub span: Span,
    pub const_values: HashMap<ValueId, LitVal>,
}

impl SsaFunction {
    pub fn new(name: String, ret_type: Option<TypeAnn>, span: Span) -> Self {
        let mut blocks = Vec::new();
        let entry_id = BlockId(0);
        blocks.push(BasicBlock::new(entry_id, "entry".to_string()));

        Self {
            name,
            params: Vec::new(),
            ret_type,
            values: Vec::new(),
            blocks,
            entry_block: entry_id,
            span,
            const_values: HashMap::new(),
        }
    }

    pub fn new_value(&mut self, name: Option<String>, span: Span) -> ValueId {
        let id = ValueId(self.values.len());
        self.values.push(ValueData { id, name, span });
        id
    }

    pub fn new_block(&mut self, label: impl Into<String>) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(BasicBlock::new(id, label.into()));
        id
    }

    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.0]
    }

    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0]
    }
}

#[derive(Debug, Clone)]
pub struct SsaProgram {
    pub functions: Vec<SsaFunction>,
    pub structs: Vec<crate::sema::ResolvedStruct>,
}
