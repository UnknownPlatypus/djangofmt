#[path = "../../../djangofmt/tests/common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{
    Applicability, FileDiagnostics, LintDiagnostic, Rule, RuleSet, Settings, fix_ast,
    graphical_handler, lint_source, parse,
};

use insta::{assert_snapshot, glob};
use markup_fmt::Language;
use miette::GraphicalTheme;
use std::fs;
use std::path::Path;
use strum::IntoEnumIterator;

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
        let ast = parse(&input, Language::Django, &[])
            .unwrap_or_else(|err| panic!("Failed to parse {}: {err:?}", path.display()));
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let settings = settings_for(path);

        let safe = fix_ast(&input, &ast, &settings, Applicability::Safe, Some(path));
        if safe.applied_count > 0 {
            build_settings(path).bind(|| {
                assert_snapshot!(format!("{stem}.fixed"), safe.output);
            });
        }

        let unsafe_fixed = fix_ast(&input, &ast, &settings, Applicability::Unsafe, Some(path));
        if unsafe_fixed.applied_count > safe.applied_count {
            build_settings(path).bind(|| {
                assert_snapshot!(format!("{stem}.unsafe-fixed"), unsafe_fixed.output);
            });
        }
    });
}

/// Every runnable rule has a non-empty fixture directory named after it.
#[test]
fn every_rule_has_a_fixture_directory() {
    for rule in Rule::iter().filter(|rule| !rule.is_deprecated() && !rule.is_removed()) {
        let code: &'static str = rule.into();
        let dir = Path::new(MANIFEST_DIR)
            .join("tests/check")
            .join(code.replace('-', "_"));
        assert!(
            fs::read_dir(&dir).is_ok_and(|mut entries| entries.next().is_some()),
            "rule `{code}` has no fixture directory, or an empty one, at {}",
            dir.display()
        );
    }
}

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// The fixture directory names the rule under test; only that rule runs, so
/// fixtures and snapshots stay local to it.
fn settings_for(path: &Path) -> Settings {
    let dir = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .expect("fixture files live in a rule directory");
    let rule = dir
        .replace('_', "-")
        .parse::<Rule>()
        .unwrap_or_else(|_| panic!("fixture directory `{dir}` does not name a rule"));
    Settings {
        rules: RuleSet::from_rule(rule),
        ..Settings::default()
    }
}

fn collect_diagnostics(path: &Path, input: &str) -> Vec<LintDiagnostic> {
    lint_source(
        input,
        Language::Django,
        &[],
        &settings_for(path),
        Some(path),
    )
    .expect("Failed to parse AST in test")
}

fn render_check_output(path: &Path, input: String, diagnostics: Vec<LintDiagnostic>) -> String {
    let display_path = path.strip_prefix(MANIFEST_DIR).unwrap_or(path);
    render_diagnostics(&FileDiagnostics::new(
        display_path.to_string_lossy(),
        input,
        diagnostics,
    ))
}

fn render_diagnostics(diagnostics: &FileDiagnostics) -> String {
    let mut output = String::new();
    diagnostics
        .render(
            &graphical_handler(GraphicalTheme::unicode_nocolor()),
            &mut output,
        )
        .expect("Failed to render diagnostics");
    output
}
