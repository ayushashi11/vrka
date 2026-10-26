use anyhow::Context;
use lang::{
    ClosureAnalysis, CodeEmitter, CompileError, TypeInferrer, build_ssa_program, parse_with_path,
};
use std::env;
use std::fs;
use std::io::{self, BufWriter, Read};
use std::process;
use vmm::bytecode::disasm::print_disassembly_with_options;
use vmm::bytecode::serde::write_bytecode;
use vmm::vm::IonVM;

fn print_usage() {
    eprintln!(
        r#"Mesh Compiler Pipeline Inspector

USAGE:
    lang [FLAGS] [OPTIONS] [FILE]

ARGS:
    <FILE>              Input source file (reads from stdin if omitted or '-')

FLAGS:
    --ast               Output the parsed Concrete Syntax Tree (CST / AST)
    --sema              Output name resolution, scopes, and variable symbols
    --mir               Output SSA + CST Mid-level Intermediate Representation
    --types             Output inferred types (row polymorphism, unboxed arrays)
    --closures          Output closure environment layout and upvalue captures
    --bytecode          Output generated IonVM bytecode instructions (including LLVM JIT IR)
    --run               Compile and execute the program in IonVM
    --all               Run and display all compilation passes in sequence
    -h, --help          Print this help message

OPTIONS:
    -o, --output <FILE> Write compiled IonVM bytecode binary (.bin) to FILE
"#
    );
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut file_path: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut show_ast = false;
    let mut show_sema = false;
    let mut show_mir = false;
    let mut show_types = false;
    let mut show_closures = false;
    let mut show_bytecode = false;
    let mut run_vm = false;
    let mut show_all = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--ast" => show_ast = true,
            "--sema" => show_sema = true,
            "--mir" => show_mir = true,
            "--types" => show_types = true,
            "--closures" => show_closures = true,
            "--bytecode" => show_bytecode = true,
            "--run" => run_vm = true,
            "--all" => show_all = true,
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(args[i].clone());
                } else {
                    anyhow::bail!("Option '{}' requires an argument", arg);
                }
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            other if !other.starts_with('-') => {
                if file_path.is_none() {
                    file_path = Some(other.to_string());
                }
            }
            "-" => {
                file_path = Some("-".to_string());
            }
            unknown => {
                anyhow::bail!(
                    "Unknown option '{}'. Use --help for usage information.",
                    unknown
                );
            }
        }
        i += 1;
    }

    // Default to --all if no specific stage flag is provided
    if !show_ast
        && !show_sema
        && !show_mir
        && !show_types
        && !show_closures
        && !show_bytecode
        && !run_vm
        && output_path.is_none()
    {
        show_all = true;
    }

    // Read source code from file or stdin
    let source_code = match file_path {
        Some(ref path) if path != "-" => fs::read_to_string(path)
            .with_context(|| format!("Failed to read source file '{}'", path))?,
        _ => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read source code from stdin")?;
            buffer
        }
    };

    if source_code.trim().is_empty() {
        anyhow::bail!("Input source code is empty");
    }

    let file_label = file_path.as_deref().unwrap_or("<stdin>");

    // ── Pass 1: Lex + Parse ──────────────────────────────────────────────────
    let mut module = match parse_with_path(&source_code, std::path::PathBuf::from(file_label)) {
        Ok(m) => m,
        Err(e) => {
            let err = CompileError::from(e);
            eprintln!("{}", err.format_with_source(&source_code, Some(file_label)));
            process::exit(1);
        }
    };
    lang::ensure_prelude_imported(&mut module);

    if show_ast || show_all {
        println!("=== [Pass 1: Parsed CST / AST] ===");
        println!("{:#?}", module);
        println!();
    }

    // ── Pass 2: Name Resolution + Scoping + Upvalues ─────────────────────────
    let file_dir = file_path.as_ref().and_then(|p| {
        let path = std::path::Path::new(p);
        path.parent().map(|d| d.to_path_buf())
    });
    let loader = lang::ModuleLoader::new(file_dir);
    let resolved = match lang::resolve_with_loader(&module, loader) {
        Ok(r) => r,
        Err(e) => {
            let err = CompileError::from(e);
            eprintln!("{}", err.format_with_source(&source_code, Some(file_label)));
            process::exit(1);
        }
    };

    if show_sema || show_all {
        println!("=== [Pass 2: Name Resolution & Scopes] ===");
        println!("Discovered Functions ({}):", resolved.functions.len());
        for f in &resolved.functions {
            println!(
                "  fn '{}' (total locals: {}, upvalues: {})",
                f.name,
                f.total_locals,
                f.upvalues.len()
            );
            for up in &f.upvalues {
                println!(
                    "    captured upvalue '{}' ({:?}, direct={})",
                    up.name, up.kind, up.is_direct_local
                );
            }
        }
        println!(
            "\nResolved Variables ({} symbols resolved):",
            resolved.var_resolutions.len()
        );
        for (span, res) in &resolved.var_resolutions {
            println!("  [{}:{}] -> {:?}", span.line, span.col, res);
        }
        println!();
    }

    // ── Pass 3: SSA + CST MIR Construction ──────────────────────────────────
    let ssa = build_ssa_program(&resolved, &module);

    if show_mir || show_all {
        println!("=== [Pass 3: SSA + CST MIR] ===");
        for func in &ssa.functions {
            print!("{}", func);
        }
        println!();
    }

    // ── Pass 4: Type Resolution (Row Polymorphism & Specialization) ──────────
    let mut inferrer = TypeInferrer::new();
    inferrer.register_program_declarations(&resolved, &module);

    let type_maps = match inferrer.infer_program(&ssa) {
        Ok(maps) => maps,
        Err(e) => {
            let err = CompileError::from(e);
            eprintln!("{}", err.format_with_source(&source_code, Some(file_label)));
            process::exit(1);
        }
    };

    if show_types || show_all {
        println!("=== [Pass 4: Inferred Types] ===");
        for func in &ssa.functions {
            println!("Function '{}':", func.name);
            if let Some(types) = type_maps.get(&func.name) {
                for (val, ty) in types {
                    println!("  {} : {}", val, ty);
                }
            }
        }
        println!();
    }

    // ── Pass 5: Closure Environment Building ─────────────────────────────────
    let closure_analysis = ClosureAnalysis::build(&resolved, &ssa, &type_maps);

    if show_closures || show_all {
        println!("=== [Pass 5: Closure Environments] ===");
        print!("{}", closure_analysis);
        println!();
    }

    // ── Pass 6: IonVM Codegen & Execution ────────────────────────────────────
    let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

    if let Some(ref out_path) = output_path {
        let file = fs::File::create(out_path)
            .with_context(|| format!("Failed to create output bytecode file '{}'", out_path))?;
        let mut writer = BufWriter::new(file);
        write_bytecode(&mut writer, &bytecode)
            .map_err(|e| anyhow::anyhow!("Failed to serialize bytecode: {}", e))?;
    }

    if show_bytecode || show_all {
        println!("=== [Pass 6: IonVM Bytecode] ===");
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = print_disassembly_with_options(&mut handle, &bytecode, true);
        println!();
    }

    if run_vm || show_all {
        println!("=== [Execution: IonVM] ===");
        lang::syntax::service::ensure_syntax_library_registered();
        match IonVM::new(bytecode) {
            Ok(mut vm) => match vm.run() {
                Ok(()) => {
                    if let Some(ret) = vm.entrypoint_return() {
                        println!("Return value: {:?}", ret);
                    } else {
                        println!("Program finished successfully (no return value).");
                    }
                }
                Err(e) => {
                    let err = CompileError::VmRuntime(e.to_string());
                    eprintln!("{}", err);
                    process::exit(1);
                }
            },
            Err(e) => {
                let err = CompileError::VmInit(e.to_string());
                eprintln!("{}", err);
                process::exit(1);
            }
        }
    }

    Ok(())
}
