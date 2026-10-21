use crate::syntax::token::Span;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SemaError {
    #[error("Undefined variable '{name}' at line {line}:{col}")]
    UndefinedVariable {
        name: String,
        line: usize,
        col: usize,
    },

    #[error("Duplicate declaration of '{name}' at line {line}:{col}")]
    DuplicateDeclaration {
        name: String,
        line: usize,
        col: usize,
    },

    #[error("Cannot reassign to constant '{name}' at line {line}:{col}")]
    CannotAssignToConstant {
        name: String,
        line: usize,
        col: usize,
    },

    #[error("Unknown namespace or module '{name}' in dotted access at line {line}:{col}")]
    UnknownNamespace {
        name: String,
        line: usize,
        col: usize,
    },

    #[error("Invalid assignment target '{target}' at line {line}:{col}")]
    InvalidAssignmentTarget {
        target: String,
        line: usize,
        col: usize,
    },

    #[error("Break statement outside of loop at line {line}:{col}")]
    BreakOutsideLoop {
        line: usize,
        col: usize,
    },

    #[error("Continue statement outside of loop at line {line}:{col}")]
    ContinueOutsideLoop {
        line: usize,
        col: usize,
    },

    #[error("Return statement outside of function at line {line}:{col}")]
    ReturnOutsideFunction {
        line: usize,
        col: usize,
    },

    #[error("Import error: {message} at line {line}:{col}")]
    ImportError {
        message: String,
        line: usize,
        col: usize,
    },

    #[error("Semantic error: {message} at line {line}:{col}")]
    Custom {
        message: String,
        line: usize,
        col: usize,
    },
}

impl SemaError {
    pub fn undefined(name: impl Into<String>, span: Span) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn duplicate(name: impl Into<String>, span: Span) -> Self {
        Self::DuplicateDeclaration {
            name: name.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn assign_constant(name: impl Into<String>, span: Span) -> Self {
        Self::CannotAssignToConstant {
            name: name.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn unknown_namespace(name: impl Into<String>, span: Span) -> Self {
        Self::UnknownNamespace {
            name: name.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn invalid_assignment_target(target: impl Into<String>, span: Span) -> Self {
        Self::InvalidAssignmentTarget {
            target: target.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn break_outside_loop(span: Span) -> Self {
        Self::BreakOutsideLoop {
            line: span.line,
            col: span.col,
        }
    }

    pub fn continue_outside_loop(span: Span) -> Self {
        Self::ContinueOutsideLoop {
            line: span.line,
            col: span.col,
        }
    }

    pub fn return_outside_function(span: Span) -> Self {
        Self::ReturnOutsideFunction {
            line: span.line,
            col: span.col,
        }
    }

    pub fn import_error(message: impl Into<String>, span: Span) -> Self {
        Self::ImportError {
            message: message.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn custom(message: impl Into<String>, span: Span) -> Self {
        Self::Custom {
            message: message.into(),
            line: span.line,
            col: span.col,
        }
    }

    pub fn line(&self) -> usize {
        match self {
            SemaError::UndefinedVariable { line, .. }
            | SemaError::DuplicateDeclaration { line, .. }
            | SemaError::CannotAssignToConstant { line, .. }
            | SemaError::UnknownNamespace { line, .. }
            | SemaError::InvalidAssignmentTarget { line, .. }
            | SemaError::BreakOutsideLoop { line, .. }
            | SemaError::ContinueOutsideLoop { line, .. }
            | SemaError::ReturnOutsideFunction { line, .. }
            | SemaError::ImportError { line, .. }
            | SemaError::Custom { line, .. } => *line,
        }
    }

    pub fn col(&self) -> usize {
        match self {
            SemaError::UndefinedVariable { col, .. }
            | SemaError::DuplicateDeclaration { col, .. }
            | SemaError::CannotAssignToConstant { col, .. }
            | SemaError::UnknownNamespace { col, .. }
            | SemaError::InvalidAssignmentTarget { col, .. }
            | SemaError::BreakOutsideLoop { col, .. }
            | SemaError::ContinueOutsideLoop { col, .. }
            | SemaError::ReturnOutsideFunction { col, .. }
            | SemaError::ImportError { col, .. }
            | SemaError::Custom { col, .. } => *col,
        }
    }
}
