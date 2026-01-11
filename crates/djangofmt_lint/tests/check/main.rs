#[path = "../../../djangofmt/tests/common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use insta::{assert_snapshot, glob};
use markup_fmt::Language;
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource};
use std::fs;
use std::path::Path;

#[test]
fn check_snapshot() {
    let pattern = "**/*.html";
    glob!(pattern, |path| {
        let input = fs::read_to_string(path).unwrap();
        let output = run_check_test(path, input);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
fn run_check_test(path: &Path, input: String) -> String {
    let mut parser = Parser::new(&input, Language::Jinja, vec![]);
    let ast = parser.parse_root().expect("Failed to parse AST in test");
    let settings = Settings::default();
    let file_diagnostics = check_ast(&ast, &settings);
    if file_diagnostics.is_empty() {
        return String::new();
    }

    let display_path = path.strip_prefix(MANIFEST_DIR).unwrap_or(path);
    render_diagnostics(&FileDiagnostics::new(
        NamedSource::new(display_path.to_string_lossy(), input),
        file_diagnostics,
    ))
}

fn render_diagnostics(diagnostics: &FileDiagnostics) -> String {
    let mut output = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .render_report(&mut output, diagnostics)
        .expect("Failed to render diagnostics");
    output
}
