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
    BasicBlock, BlockId, Phi, SsaFunction, SsaInst, SsaMatchArm, SsaOp, SsaProgram, SsaTerminator,
    ValueData, ValueId, build_ssa_program,
};
pub use sema::{
    ModuleLoader, ResolvedEnum, ResolvedFunction, ResolvedProgram, ResolvedStruct, ResolvedVar,
    SemaError, VarKind, resolve, resolve_with_loader,
};
pub use syntax::{Module, ParseError, parse, parse_expression, parse_with_path};
pub use types::{RowVarId, Type, TypeEnv, TypeError, TypeInferrer, TypeVarId};

pub const PRELUDE_SOURCE: &str = include_str!("../stdlib/prelude.mesh");

pub fn ensure_prelude_imported(module: &mut syntax::Module) {
    if module.path == std::path::Path::new("__ionvm.prelude") {
        return;
    }
    let already_imported = module.items.iter().any(|item| {
        if let syntax::Item::Decl(syntax::Decl::Import(imp)) = item {
            match &imp.target {
                syntax::ImportTarget::Path(p) | syntax::ImportTarget::Ident(p) => {
                    p == "__ionvm.prelude"
                }
                _ => false,
            }
        } else {
            false
        }
    });
    if !already_imported {
        module.items.insert(
            0,
            syntax::Item::Decl(syntax::Decl::Import(syntax::ImportDef {
                target: syntax::ImportTarget::Path("__ionvm.prelude".to_string()),
                span: syntax::Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    col: 1,
                },
            })),
        );
    }
}

/// Compile source code into IonVM Bytecode
pub fn compile(
    source: &str,
) -> std::result::Result<vmm::bytecode::instruction::Bytecode, CompileError> {
    compile_with_path(source, None)
}

/// Compile source code into IonVM Bytecode with a base file path for relative imports
pub fn compile_with_path(
    source: &str,
    file_path: Option<&std::path::Path>,
) -> std::result::Result<vmm::bytecode::instruction::Bytecode, CompileError> {
    let mut module = parse_with_path(
        source,
        file_path
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("<memory>")),
    )?;
    ensure_prelude_imported(&mut module);
    let loader =
        sema::ModuleLoader::new(file_path.and_then(|p| p.parent().map(|d| d.to_path_buf())));
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
    vm.run()
        .map_err(|e| anyhow::anyhow!("IonVM runtime error: {}", e))?;
    Ok(vm.entrypoint_return())
}

#[cfg(test)]
mod tests {
    use super::{compile, run};
    use vmm::bytecode::instruction::OperandType;
    use vmm::bytecode::opcode::Opcode;
    use vmm::lang::types::Value;

    #[test]
    fn primitive_to_string_calls_mesh_method_with_format_opcode() {
        let bytecode = compile(
            r#"
            fn main() -> String:
                return true.to_string()
            "#,
        )
        .expect("program compiles");

        let main = bytecode
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function is emitted");
        assert!(main.instructions.iter().any(|instruction| {
            instruction.opcode == Opcode::GetProp
                && matches!(
                    instruction.operands.get(1),
                    Some(OperandType::Property(property)) if property == "to_string"
                )
        }));
        assert!(
            main.instructions
                .iter()
                .any(|instruction| instruction.opcode == Opcode::Call)
        );

        let to_string = bytecode
            .functions
            .iter()
            .find(|function| function.name == "__ionvm.prelude.Bool.#ToString.to_string")
            .expect("Bool ToString implementation is emitted separately");
        assert!(
            to_string
                .instructions
                .iter()
                .any(|instruction| instruction.opcode == Opcode::FormatValue)
        );
    }

    #[test]
    fn primitive_to_string_uses_mesh_prelude_implementations() {
        let result = run(r#"
            fn main() -> Array[String]:
                return [42.to_string(), 3.14.to_string(), true.to_string(), "mesh".to_string()]
            "#)
        .expect("program runs");

        assert_eq!(
            result,
            Some(Value::Array(std::sync::Arc::new(std::sync::RwLock::new(
                vec![
                    Value::String("42".into()),
                    Value::String("3.14".into()),
                    Value::String("true".into()),
                    Value::String("mesh".into()),
                ]
            ))))
        );
    }
}
