use crate::mir::ssa::{BlockId, ValueId};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CodegenError {
    #[error("Missing destination register for instruction: {0}")]
    MissingDestination(String),

    #[error("Target block '{0:?}' not found during branch / jump emission")]
    BlockNotFound(BlockId),

    #[error("Value '{0:?}' was not allocated a register")]
    UnallocatedRegister(ValueId),

    #[error("Codegen error: {0}")]
    Custom(String),
}
