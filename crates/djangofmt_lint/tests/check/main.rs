#![allow(clippy::unwrap_used)]
#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{LintDiagnostic, check_ast};
use insta::{assert_snapshot, glob};
use markup_fmt::Language;
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme};
use std::{fs, path::Path};

#[test]
fn check_snapshot() {
    let pattern = "**/*.html";
    glob!(pattern, |path| {
        let input = fs::read_to_string(path).unwrap();
        let output = run_check_test(path, &input);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

fn run_check_test(_path: &Path, input: &str) -> String {
    let mut parser = Parser::new(input, Language::Jinja, vec![]);
    let ast = parser.parse_root().unwrap();
    let diagnostics = check_ast(input, &ast);

    if diagnostics.is_empty() {
        return String::new();
    }

    render_diagnostics(&diagnostics)
}

fn render_diagnostics(diagnostics: &[LintDiagnostic]) -> String {
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let mut output = String::new();

    for diag in diagnostics {
        handler.render_report(&mut output, diag).unwrap();
    }

    output
}
