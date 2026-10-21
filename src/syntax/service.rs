//! First-class Syntax & Tooling Library for Mesh
//! Exposes lexer, layout preprocessor, parser, type checker, and formatter
//! directly to Mesh scripts via FFI.

use std::collections::HashMap;
use vm_ffi::{FfiError, FfiFunction, FfiRegistry, FfiResult, FfiType, FfiValue, RawFfiFunction};
use crate::syntax::lexer::Lexer;
use crate::syntax::token::TokenKind;
use crate::syntax::{layout, parse};
use crate::{build_ssa_program, resolve, TypeInferrer};

macro_rules! syntax_ffi_function {
    ($name:ident, $arity:expr, $ret_type:expr, $description:expr, |$args:ident| $body:expr) => {
        pub struct $name;

        impl RawFfiFunction for $name {
            fn call(&self, _ctx: &mut dyn vm_ffi::FfiContext, args: Vec<FfiValue>) -> FfiResult {
                if args.len() != $arity {
                    return Err(FfiError::ArgumentCount {
                        expected: $arity,
                        got: args.len(),
                    });
                }
                let handler = |$args: Vec<FfiValue>| -> Result<FfiValue, FfiError> { $body };
                handler(args).map(Into::into)
            }

            fn name(&self) -> &str {
                stringify!($name)
            }

            fn arity(&self) -> usize {
                $arity
            }

            fn return_type(&self) -> FfiType {
                $ret_type
            }

            fn description(&self) -> Option<&str> {
                Some($description)
            }
        }

        impl FfiFunction for $name {}
    };
}

fn extract_string(val: &FfiValue) -> Result<String, FfiError> {
    match val {
        FfiValue::String(s) => Ok(s.clone()),
        FfiValue::Atom(s) => Ok(s.clone()),
        other => Err(FfiError::ArgumentType {
            expected: "String".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

// ── 1. mesh_lex(source: String) -> Array<TokenObject> ────────────────────────
syntax_ffi_function!(MeshLex, 1, FfiType::Array, "Lexes Mesh source into an array of token objects", |args| {
    let src = extract_string(&args[0])?;
    let mut lexer = Lexer::new(&src);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == TokenKind::Eof;

        let mut map = HashMap::new();
        map.insert("kind".to_string(), FfiValue::String(format!("{:?}", tok.kind)));
        map.insert("line".to_string(), FfiValue::Integer(tok.span.line as i64));
        map.insert("col".to_string(), FfiValue::Integer(tok.span.col as i64));
        map.insert("start".to_string(), FfiValue::Integer(tok.span.start as i64));
        map.insert("end".to_string(), FfiValue::Integer(tok.span.end as i64));

        let text = if tok.span.end <= src.len() && tok.span.start <= tok.span.end {
            src[tok.span.start..tok.span.end].to_string()
        } else {
            String::new()
        };
        map.insert("text".to_string(), FfiValue::String(text));

        tokens.push(FfiValue::Object(map));
        if is_eof {
            break;
        }
    }

    Ok(FfiValue::Array(tokens))
});

// ── 2. mesh_layout(source: String) -> String ─────────────────────────────────
syntax_ffi_function!(MeshLayout, 1, FfiType::String, "Preprocesses indentation layout into explicit block braces", |args| {
    let src = extract_string(&args[0])?;
    let laid_out = layout::layout(&src);
    Ok(FfiValue::String(laid_out))
});

// ── 3. mesh_check(source: String) -> Array<Diagnostic> ───────────────────────
syntax_ffi_function!(MeshCheck, 1, FfiType::Array, "Checks Mesh source for parse, semantic, or type errors", |args| {
    let src = extract_string(&args[0])?;
    let mut diagnostics = Vec::new();

    // Pass 1: Parse
    let module = match parse(&src) {
        Ok(m) => m,
        Err(e) => {
            let mut map = HashMap::new();
            map.insert("message".to_string(), FfiValue::String(e.message));
            map.insert("line".to_string(), FfiValue::Integer(e.span.line as i64));
            map.insert("col".to_string(), FfiValue::Integer(e.span.col as i64));
            map.insert("severity".to_string(), FfiValue::Integer(1)); // Error
            diagnostics.push(FfiValue::Object(map));
            return Ok(FfiValue::Array(diagnostics));
        }
    };

    // Pass 2: Semantic name resolution
    let resolved = match resolve(&module) {
        Ok(r) => r,
        Err(e) => {
            let mut map = HashMap::new();
            map.insert("message".to_string(), FfiValue::String(e.to_string()));
            map.insert("line".to_string(), FfiValue::Integer(e.line() as i64));
            map.insert("col".to_string(), FfiValue::Integer(e.col() as i64));
            map.insert("severity".to_string(), FfiValue::Integer(1));
            diagnostics.push(FfiValue::Object(map));
            return Ok(FfiValue::Array(diagnostics));
        }
    };

    // Pass 3: MIR & Type inference
    let ssa = build_ssa_program(&resolved, &module);
    let mut inferrer = TypeInferrer::new();
    inferrer.register_program_declarations(&resolved, &module);
    if let Err(e) = inferrer.infer_program(&ssa) {
        let mut map = HashMap::new();
        map.insert("message".to_string(), FfiValue::String(e.to_string()));
        map.insert("line".to_string(), FfiValue::Integer(1));
        map.insert("col".to_string(), FfiValue::Integer(1));
        map.insert("severity".to_string(), FfiValue::Integer(1));
        diagnostics.push(FfiValue::Object(map));
    }

    Ok(FfiValue::Array(diagnostics))
});

// ── 4. mesh_format(source: String) -> String ─────────────────────────────────
syntax_ffi_function!(MeshFormat, 1, FfiType::String, "Formats Mesh source code with standard 4-space indentation and spacing", |args| {
    let src = extract_string(&args[0])?;
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out = String::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut pending_indent = false;
    let mut prev_blank = false;

    for line in lines {
        let expanded = line.replace('\t', "    ");
        let trimmed = expanded.trim();
        if trimmed.is_empty() {
            if !prev_blank && !out.is_empty() {
                out.push('\n');
                prev_blank = true;
            }
            continue;
        }
        prev_blank = false;

        let leading = expanded.len() - expanded.trim_start().len();

        // Top-level declarations always reset indent to 0
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("protocol ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("import ")
        {
            indent_stack.truncate(1);
            pending_indent = false;
        } else if pending_indent {
            if leading > *indent_stack.last().unwrap_or(&0) {
                indent_stack.push(leading);
            }
            pending_indent = false;
        } else {
            while indent_stack.len() > 1 && leading < *indent_stack.last().unwrap_or(&0) {
                indent_stack.pop();
            }
        }

        let depth = indent_stack.len().saturating_sub(1);

        // Dedent keyword 'else:' / 'elif ' by 1 if aligned with inner block
        if (trimmed.starts_with("else:") || trimmed.starts_with("elif ")) && depth > 0 && leading <= *indent_stack.last().unwrap_or(&0) {
            // keep depth at matching if level
        }

        // Spacing normalization outside quotes
        let mut formatted = String::new();
        let mut in_str = false;
        let mut quote_char = ' ';

        for ch in trimmed.chars() {
            if !in_str && (ch == '"' || ch == '\'') {
                in_str = true;
                quote_char = ch;
                formatted.push(ch);
            } else if in_str && ch == quote_char {
                in_str = false;
                formatted.push(ch);
            } else if in_str {
                formatted.push(ch);
            } else {
                formatted.push(ch);
            }
        }

        // Normalize spaces around commas outside strings
        formatted = formatted.replace(" ,", ",");
        formatted = formatted.replace(',', ", ");
        while formatted.contains(",  ") {
            formatted = formatted.replace(",  ", ", ");
        }

        // Normalize arrows
        formatted = formatted.replace(" ->", " -> ");
        formatted = formatted.replace("-> ", " -> ");
        while formatted.contains("  ->  ") {
            formatted = formatted.replace("  ->  ", " -> ");
        }

        // Normalize colons in type annotations and blocks
        formatted = formatted.replace(" :", ":");
        formatted = formatted.replace(':', ": ");
        while formatted.contains(":  ") {
            formatted = formatted.replace(":  ", ": ");
        }
        if formatted.ends_with(": ") {
            formatted.pop();
        }
        // Preserve atom variants (:Some, :None, etc.)
        formatted = formatted.replace(": Some", ":Some");
        formatted = formatted.replace(": None", ":None");
        formatted = formatted.replace(": Ok", ":Ok");
        formatted = formatted.replace(": Err", ":Err");
        formatted = formatted.replace("Option: Some", "Option:Some");
        formatted = formatted.replace("Result: Ok", "Result:Ok");
        formatted = formatted.replace("Result: Err", "Result:Err");

        // Binary operators spacing
        formatted = formatted.replace(" = ", "=");
        formatted = formatted.replace('=', " = ");
        formatted = formatted.replace(" == ", "==");
        formatted = formatted.replace("==", " == ");
        formatted = formatted.replace(" != ", "!=");
        formatted = formatted.replace("!=", " != ");
        formatted = formatted.replace(" <= ", "<=");
        formatted = formatted.replace("<=", " <= ");
        formatted = formatted.replace(" >= ", ">=");
        formatted = formatted.replace(">=", " >= ");
        formatted = formatted.replace(" < ", "<");
        formatted = formatted.replace('<', " < ");
        formatted = formatted.replace("< =", "<=");
        formatted = formatted.replace("<=", " <= ");
        formatted = formatted.replace(" > ", ">");
        formatted = formatted.replace('>', " > ");
        formatted = formatted.replace("> =", ">=");
        formatted = formatted.replace(">=", " >= ");
        formatted = formatted.replace("- >", " -> ");
        formatted = formatted.replace(" + ", "+");
        formatted = formatted.replace('+', " + ");
        while formatted.contains("  ") {
            formatted = formatted.replace("  ", " ");
        }
        let formatted = formatted.trim();

        let prefix = "    ".repeat(depth);
        out.push_str(&prefix);
        out.push_str(formatted);
        out.push('\n');

        if trimmed.ends_with(':') || trimmed.ends_with('{') {
            pending_indent = true;
        }
    }

    Ok(FfiValue::String(out))
});

// ── 5. mesh_parse_ast(source: String) -> String ──────────────────────────────
syntax_ffi_function!(MeshParseAst, 1, FfiType::String, "Parses source code and returns the CST debug representation", |args| {
    let src = extract_string(&args[0])?;
    match parse(&src) {
        Ok(module) => Ok(FfiValue::String(format!("{:#?}", module))),
        Err(e) => Err(FfiError::RuntimeError(format!("Parse error at {}:{}: {}", e.span.line, e.span.col, e.message))),
    }
});

/// Register all syntax tools in the FFI registry
pub fn register_syntax_library(registry: &mut FfiRegistry) {
    registry.register(MeshLex);
    registry.register(MeshLayout);
    registry.register(MeshCheck);
    registry.register(MeshFormat);
    registry.register(MeshParseAst);
}

/// Direct FFI registration callback
pub unsafe extern "Rust" fn register_syntax_library_ffi(registry: &mut FfiRegistry) {
    register_syntax_library(registry);
}

/// Helper to ensure all VM worker threads have the syntax library loaded
pub fn ensure_syntax_library_registered() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        vmm::builtin::native_lib::register_rust_plugin(register_syntax_library_ffi);
    });
}
