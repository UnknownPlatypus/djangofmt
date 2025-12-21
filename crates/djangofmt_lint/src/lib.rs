mod checker;
mod rules;

pub use checker::Checker;

use markup_fmt::ast::Root;
use miette::{Diagnostic, SourceSpan};

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("{message}")]
pub struct LintDiagnostic {
    #[source_code]
    pub source_code: String,
    pub code: &'static str,
    pub message: String,
    #[label("{label}")]
    pub span: SourceSpan,
    pub label: String,
    #[help]
    pub help: Option<String>,
}

/// Check the AST for lint errors.
pub fn check_ast(source: &str, ast: &Root<'_>) -> Vec<LintDiagnostic> {
    let mut checker = Checker::new(source);
    checker.visit_root(ast);
    checker.into_diagnostics()
}
