use crate::codegen::CodegenError;
use crate::sema::SemaError;
use crate::syntax::ParseError;
use crate::types::TypeError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompileError>;

/// Unified compilation error for the Mesh language pipeline
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Syntax error: {0}")]
    Parse(#[from] ParseError),

    #[error("Semantic error: {0}")]
    Sema(#[from] SemaError),

    #[error("Type error: {0}")]
    Type(#[from] TypeError),

    #[error("Codegen error: {0}")]
    Codegen(#[from] CodegenError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("VM initialization error: {0}")]
    VmInit(String),

    #[error("VM runtime error: {0}")]
    VmRuntime(String),
}

impl CompileError {
    /// Extract line and column information if available for diagnostic display
    pub fn location(&self) -> Option<(usize, usize)> {
        match self {
            CompileError::Parse(e) => Some((e.span.line, e.span.col)),
            CompileError::Sema(e) => match e {
                SemaError::UndefinedVariable { line, col, .. } => Some((*line, *col)),
                SemaError::DuplicateDeclaration { line, col, .. } => Some((*line, *col)),
                SemaError::CannotAssignToConstant { line, col, .. } => Some((*line, *col)),
                SemaError::UnknownNamespace { line, col, .. } => Some((*line, *col)),
                SemaError::InvalidAssignmentTarget { line, col, .. } => Some((*line, *col)),
                SemaError::BreakOutsideLoop { line, col } => Some((*line, *col)),
                SemaError::ContinueOutsideLoop { line, col } => Some((*line, *col)),
                SemaError::ReturnOutsideFunction { line, col } => Some((*line, *col)),
                SemaError::ImportError { line, col, .. } => Some((*line, *col)),
                SemaError::Custom { line, col, .. } => Some((*line, *col)),
            },
            _ => None,
        }
    }

    /// Render compiler error with source context and caret pointers
    pub fn format_with_source(&self, source: &str, file_name: Option<&str>) -> String {
        let file_label = file_name.unwrap_or("<input>");
        if let Some((line, col)) = self.location() {
            let lines: Vec<&str> = source.lines().collect();
            if line > 0 && line <= lines.len() {
                let src_line = lines[line - 1];
                let col_offset = if col > 0 { col - 1 } else { 0 };
                let pointer = format!("{}^", " ".repeat(col_offset));
                return format!(
                    "{}\n  --> {}:{}:{}\n   |\n{:4} | {}\n   | {}",
                    self, file_label, line, col, line, src_line, pointer
                );
            }
        }
        format!("{}: {}", file_label, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::token::Span;

    #[test]
    fn test_parse_error_conversion_and_formatting() {
        let span = Span::new(10, 15, 2, 5);
        let parse_err = ParseError::new("Expected token ';'", span);
        let compile_err = CompileError::from(parse_err);

        assert_eq!(compile_err.location(), Some((2, 5)));

        let src = "let a = 1\nlet b = 2\nlet c = 3";
        let formatted = compile_err.format_with_source(src, Some("test.mesh"));
        assert!(formatted.contains("test.mesh:2:5"));
        assert!(formatted.contains("let b = 2"));
        assert!(formatted.contains("    ^"));
    }

    #[test]
    fn test_sema_error_conversion_and_formatting() {
        let span = Span::new(0, 5, 1, 10);
        let sema_err = SemaError::undefined("foo", span);
        let compile_err = CompileError::from(sema_err);

        assert_eq!(compile_err.location(), Some((1, 10)));

        let src = "let x = foo + 1";
        let formatted = compile_err.format_with_source(src, Some("main.mesh"));
        assert!(formatted.contains("main.mesh:1:10"));
        assert!(formatted.contains("Undefined variable 'foo'"));
    }

    #[test]
    fn test_type_error_conversion() {
        let type_err = TypeError::ArityMismatch {
            name: "sqrt".to_string(),
            expected: 1,
            found: 3,
        };
        let compile_err = CompileError::from(type_err);
        let msg = compile_err.to_string();
        assert!(msg.contains("sqrt"));
        assert!(msg.contains("expected 1 arguments, but received 3"));
    }

    #[test]
    fn test_codegen_error_conversion() {
        let codegen_err = CodegenError::MissingDestination("Add".to_string());
        let compile_err = CompileError::from(codegen_err);
        let msg = compile_err.to_string();
        assert!(msg.contains("Missing destination register"));
    }
}
