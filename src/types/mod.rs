pub mod infer;
pub mod ty;

pub use infer::{TypeEnv, TypeError, TypeInferrer};
pub use ty::{RowVarId, Type, TypeVarId};
