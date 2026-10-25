pub mod builder;
pub mod display;
pub mod ssa;

pub use builder::{SsaBuilder, build_ssa_program};
pub use ssa::{
    BasicBlock, BlockId, Phi, SsaFunction, SsaInst, SsaMatchArm, SsaOp, SsaProgram, SsaTerminator,
    ValueData, ValueId,
};
