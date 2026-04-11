#[path = "../../../djangofmt/tests/common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{FileDiagnostics, LintDiagnostic, Settings, check_ast};

use insta::{assert_snapshot, glob};
use markup_fmt::Language;
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource};
use std::fs;
use std::path::Path;

#[test]
fn check_invalid() {
    glob!("**/*.invalid.html", |path| {
        let input = fs::read_to_string(path).unwrap();
        let output = run_check_test(path, input);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

#[test]
fn check_valid() {
    glob!("**/*.valid.html", |path| {
        build_settings(path).bind(|| {
            let input = fs::read_to_string(path).unwrap();
            let mut parser = Parser::new(&input, Language::Jinja, vec![]);
            let ast = parser.parse_root().expect("Failed to parse AST in test");
            let settings = Settings::default();
            let diagnostics = check_ast(&input, &ast, &settings);
            assert!(
                diagnostics.is_empty(),
                "Expected no diagnostics for {}, but found {}:\n{}",
                path.display(),
                diagnostics.len(),
                render_check_output(path, input, diagnostics),
            );
        });
    });
}

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn run_check_test(path: &Path, input: String) -> String {
    let mut parser = Parser::new(&input, Language::Jinja, vec![]);
    let ast = parser.parse_root().expect("Failed to parse AST in test");
    let settings = Settings::default();
    let file_diagnostics = check_ast(&input, &ast, &settings);
    if file_diagnostics.is_empty() {
        return String::new();
    }

    render_check_output(path, input, file_diagnostics)
}

fn render_check_output(path: &Path, input: String, diagnostics: Vec<LintDiagnostic>) -> String {
    let display_path = path.strip_prefix(MANIFEST_DIR).unwrap_or(path);
    render_diagnostics(&FileDiagnostics::new(
        NamedSource::new(display_path.to_string_lossy(), input),
        diagnostics,
    ))
}

fn render_diagnostics(diagnostics: &FileDiagnostics) -> String {
    let mut output = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .render_report(&mut output, diagnostics)
        .expect("Failed to render diagnostics");
    output
}
