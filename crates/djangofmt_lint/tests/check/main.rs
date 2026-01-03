#![allow(clippy::unwrap_used)]
#[path = "../../../djangofmt/tests/common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
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
        let output = run_check_test(path, input);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

/// Run the checker on the given template input and return a rendered diagnostics report.
///
/// If no diagnostics are produced, an empty string is returned.
///
/// # Returns
///
/// A string containing formatted diagnostics for the input; an empty string if there are no diagnostics.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// let input = "{{ invalid jinja".to_string();
/// let out = run_check_test(Path::new("templates/example.html"), input);
/// // `out` is either an empty string or contains a human-readable diagnostics report.
/// ```
fn run_check_test(_path: &Path, input: String) -> String {
    let mut parser = Parser::new(&input, Language::Jinja, vec![]);
    let ast = parser.parse_root().unwrap();
    let settings = Settings::default();
    let file_diagnostics = check_ast(&ast, &settings);

    if file_diagnostics.is_empty() {
        return String::new();
    }
    render_diagnostics(&FileDiagnostics::new(input, file_diagnostics))
}

/// Render a FileDiagnostics value into a human-readable report string.
///
/// This uses a GraphicalReportHandler with the `unicode_nocolor` theme to format
/// the diagnostics into a single String suitable for snapshots or CLI output.
///
/// # Examples
///
/// ```no_run
/// # use djangofmt_lint::FileDiagnostics;
/// # // Construct or obtain `diagnostics` from your lint run.
/// # let diagnostics: FileDiagnostics = Default::default();
/// let report = render_diagnostics(&diagnostics);
/// // `report` contains the formatted diagnostics report (may be empty).
/// println!("{}", report);
/// ```
fn render_diagnostics(diagnostics: &FileDiagnostics) -> String {
    let mut output = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .render_report(&mut output, diagnostics)
        .unwrap();
    output
}
