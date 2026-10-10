#[path = "../../../djangofmt/tests/common.rs"]
mod common;

use common::build_settings;
use djangofmt_lint::{
    Applicability, FileDiagnostics, LintDiagnostic, Rule, RuleSet, Settings, apply_fixes, check,
    lint_source,
};
use djangofmt_syntax::{Profile, graphical_handler, parse};

use insta::{assert_snapshot, glob};
use markup_fmt::Language;
use miette::GraphicalTheme;
use std::fs;
use std::path::Path;
use strum::IntoEnumIterator;

/// Asserts every `*.valid.html` fixture produces zero diagnostics.
#[test]
fn check_valid() {
    glob!("**/*.valid.{html,jinja}", |path| {
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
    glob!("**/*.invalid.{html,jinja}", |path| {
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
    glob!("**/*.invalid.{html,jinja}", |path| {
        let input = fs::read_to_string(path).unwrap();
        let parsed = parse(&input, language_for(path), &[])
            .unwrap_or_else(|err| panic!("Failed to parse {}: {err:?}", path.display()));
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let settings = settings_for(path);

        let diagnostics = check(&parsed, &settings, Some(path));
        let fix = |threshold| apply_fixes(&input, &diagnostics, threshold);

        let safe = fix(Applicability::Safe);
        let unsafe_fixed = fix(Applicability::Unsafe);
        assert_fix_snapshot(
            path,
            &format!("{stem}.fixed"),
            safe.applied_count > 0,
            &safe.output,
        );
        assert_fix_snapshot(
            path,
            &format!("{stem}.unsafe-fixed"),
            unsafe_fixed.applied_count > safe.applied_count,
            &unsafe_fixed.output,
        );
    });
}

/// A fix that stops applying leaves its snapshot unreferenced, which only
/// `cargo insta test --unreferenced reject` reports, so check for it here.
fn assert_fix_snapshot(path: &Path, name: &str, applied: bool, output: &str) {
    if applied {
        build_settings(path).bind(|| assert_snapshot!(name, output));
    } else {
        let snapshot = path.with_file_name(format!("{name}.snap"));
        assert!(
            !snapshot.exists(),
            "{} expects a fix, but none applied",
            snapshot.display()
        );
    }
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
    let mut rules = RuleSet::from_rule(rule);
    // `unused-ignore-code` judges what other rules reported, so its fixtures also run the two
    // rules their comments list; a comment naming any other rule stands for a non-enabled one.
    if rule == Rule::UnusedIgnoreCode {
        rules.insert(Rule::InvalidAttrValue);
        rules.insert(Rule::EmptyAttrValue);
    }
    Settings {
        rules,
        ..Settings::default()
    }
}

fn collect_diagnostics(path: &Path, input: &str) -> Vec<LintDiagnostic> {
    lint_source(
        input,
        language_for(path),
        &[],
        &settings_for(path),
        Some(path),
    )
    .expect("Failed to parse AST in test")
}

/// `.jinja` fixtures run under Jinja, the only profile allowing whitespace-control markers.
fn language_for(path: &Path) -> Language {
    Profile::from_path(path).unwrap_or_default().into()
}

fn render_check_output(path: &Path, input: String, diagnostics: Vec<LintDiagnostic>) -> String {
    // `glob!` yields canonical paths, `\\?\`-prefixed on Windows.
    let manifest_dir = Path::new(MANIFEST_DIR)
        .canonicalize()
        .expect("CARGO_MANIFEST_DIR exists");
    let display_path = path.strip_prefix(manifest_dir).unwrap_or(path);
    render_diagnostics(&FileDiagnostics::new(
        display_path.to_string_lossy().replace('\\', "/"),
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
