#[path = "../../../djangofmt/tests/common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{
    Applicability, FileDiagnostics, LintDiagnostic, Rule, RuleSet, Settings, check_ast, fix_ast,
};

use insta::{assert_snapshot, glob};
use markup_fmt::Language;
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Asserts every `*.valid.html` fixture produces zero diagnostics.
#[test]
fn check_valid() {
    glob!("**/*.valid.html", |path| {
        build_settings(path).bind(|| {
            let input = fs::read_to_string(path).unwrap();
            let diagnostics = collect_diagnostics(path, &input);
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

/// Snapshots the rendered diagnostics produced for each `*.invalid.html` fixture.
#[test]
fn check_invalid() {
    glob!("**/*.invalid.html", |path| {
        let input = fs::read_to_string(path).unwrap();
        let file_diagnostics = collect_diagnostics(path, &input);
        assert!(
            !file_diagnostics.is_empty(),
            "Expected diagnostics, got none"
        );
        let output = render_check_output(path, input, file_diagnostics);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

/// Snapshots the post-fix source for each `*.invalid.html` fixture that produces a fix.
///
/// Safe fixes are snapshot as `{stem}.fixed`;
/// Unsafe fixes are snapshot as `{stem}.unsafe-fixed`;
#[test]
fn fix_snapshot() {
    glob!("**/*.invalid.html", |path| {
        let input = fs::read_to_string(path).unwrap();
        let mut parser = Parser::new(&input, Language::Django, vec![]);
        let ast = parser
            .parse_root()
            .unwrap_or_else(|err| panic!("Failed to parse {}: {err:?}", path.display()));
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let settings = fixture_settings(path);

        let safe = fix_ast(&input, &ast, &settings, Applicability::Safe);
        if safe.applied_count > 0 {
            build_settings(path).bind(|| {
                assert_snapshot!(format!("{stem}.fixed"), safe.output);
            });
        }

        let unsafe_fixed = fix_ast(&input, &ast, &settings, Applicability::Unsafe);
        if unsafe_fixed.applied_count > safe.applied_count {
            build_settings(path).bind(|| {
                assert_snapshot!(format!("{stem}.unsafe-fixed"), unsafe_fixed.output);
            });
        }
    });
}

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Settings enabling only the rule named by the fixture's directory, so each
/// fixture exercises its own rule without dodging every other one.
fn fixture_settings(path: &Path) -> Settings {
    let dir = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .expect("fixture path must have a parent directory");
    let rule = Rule::from_str(&dir.replace('_', "-"))
        .unwrap_or_else(|_| panic!("fixture directory `{dir}` does not name a rule"));
    Settings {
        rules: RuleSet::from_rule(rule),
    }
}

fn collect_diagnostics(path: &Path, input: &str) -> Vec<LintDiagnostic> {
    let mut parser = Parser::new(input, Language::Django, vec![]);
    let ast = parser.parse_root().expect("Failed to parse AST in test");
    check_ast(input, &ast, &fixture_settings(path))
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
