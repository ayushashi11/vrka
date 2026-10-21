pub mod builtins;
pub mod closure;
pub mod codegen;
pub mod error;
pub mod mir;
pub mod sema;
pub mod syntax;
pub mod types;

pub use builtins::{BuiltinFn, Builtins};
pub use closure::{CaptureKind, ClosureAnalysis, ClosureCapture, ClosureEnv};
pub use codegen::{CodeEmitter, CodegenError, RegAlloc};
pub use error::{CompileError, Result};
pub use mir::{
    build_ssa_program, BasicBlock, BlockId, Phi, SsaFunction, SsaInst, SsaMatchArm, SsaOp,
    SsaProgram, SsaTerminator, ValueData, ValueId,
};
pub use sema::{resolve, resolve_with_loader, ModuleLoader, ResolvedFunction, ResolvedProgram, ResolvedVar, SemaError, VarKind};
pub use syntax::{parse, parse_expression, Module, ParseError};
pub use types::{RowVarId, Type, TypeEnv, TypeError, TypeInferrer, TypeVarId};

/// Compile source code into IonVM Bytecode
pub fn compile(source: &str) -> std::result::Result<vmm::bytecode::instruction::Bytecode, CompileError> {
    compile_with_path(source, None)
}

/// Compile source code into IonVM Bytecode with a base file path for relative imports
pub fn compile_with_path(
    source: &str,
    file_path: Option<&std::path::Path>,
) -> std::result::Result<vmm::bytecode::instruction::Bytecode, CompileError> {
    let module = parse(source)?;
    let loader = sema::ModuleLoader::new(file_path.and_then(|p| p.parent().map(|d| d.to_path_buf())));
    let resolved = sema::resolve_with_loader(&module, loader)?;
    let ssa = build_ssa_program(&resolved, &module);
    let mut inferrer = TypeInferrer::new();
    inferrer.register_program_declarations(&resolved, &module);
    let type_maps = inferrer.infer_program(&ssa)?;
    let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));
    Ok(bytecode)
}

/// Compile and execute source code directly in IonVM using anyhow for context-rich errors
pub fn run(source: &str) -> anyhow::Result<Option<vmm::lang::types::Value>> {
    use anyhow::Context;
    syntax::service::ensure_syntax_library_registered();
    let bytecode = compile(source)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to compile source program")?;
    let mut vm = vmm::vm::IonVM::new(bytecode)
        .map_err(|e| anyhow::anyhow!("IonVM initialization error: {}", e))?;
    vm.run().map_err(|e| anyhow::anyhow!("IonVM runtime error: {}", e))?;
    Ok(vm.entrypoint_return())
}
