use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;
use tempfile::TempDir;

fn cli() -> Command {
    Command::new(get_cargo_bin("djangofmt"))
}

// ── Format subcommand ────────────────────────────────────────────────

#[test]
fn format_single_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<div   class=\"foo\"  >\n</div>\n").unwrap();
    assert_cmd_snapshot!(cli().arg(file.as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    1 file reformatted !

    ----- stderr -----
    "#);
    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "<div class=\"foo\"></div>\n");
}

#[test]
fn format_already_formatted_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<div class=\"foo\"></div>\n").unwrap();
    assert_cmd_snapshot!(cli().arg(file.as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    1 file left unchanged !

    ----- stderr -----
    "#);
}

#[test]
fn format_file_with_ignore_directive() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    let original = "<!-- djangofmt:ignore -->\n<div   class=\"foo\"  ></div>\n";
    std::fs::write(&file, original).unwrap();
    assert_cmd_snapshot!(cli().arg(file.as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    1 file skipped !

    ----- stderr -----
    "#);
    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, original);
}

#[test]
fn format_nonexistent_file() {
    assert_cmd_snapshot!(cli().arg("/nonexistent/path.html"), @r#"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    djangofmt failed
      Error: Path does not exist: /nonexistent/path.html
    "#);
}

#[test]
fn format_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.html"), "<div   ></div>\n").unwrap();
    std::fs::write(dir.path().join("b.html"), "<span   ></span>\n").unwrap();
    assert_cmd_snapshot!(cli().arg(dir.path().as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    2 files reformatted !

    ----- stderr -----
    "#);
}

#[test]
fn format_quiet() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<div   ></div>\n").unwrap();
    assert_cmd_snapshot!(cli().arg("-q").arg(file.as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "#);
}

// ── Check subcommand ─────────────────────────────────────────────────

#[test]
fn check_clean_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<form method=\"post\"></form>\n").unwrap();
    assert_cmd_snapshot!(cli().arg("check").arg(file.as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "#);
}

#[test]
fn check_file_with_lint_error() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<form method=\"put\"></form>\n").unwrap();
    let output = cli().arg("check").arg(file.as_os_str()).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid value 'put' for attribute 'method'"));
    assert!(stdout.contains("Use one of: get, post, dialog"));
}

#[test]
fn check_nonexistent_file() {
    assert_cmd_snapshot!(cli().args(["check", "/nonexistent/path.html"]), @r#"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    djangofmt failed
      Error: Path does not exist: /nonexistent/path.html
    "#);
}

// ── Rule selection (--select / --ignore / --preview) ───────────────

/// Write the canonical `<form method="put">...` 5-violation fixture and
/// return its path. Tests can run any selector combination against it.
fn write_invalid_form_method_file(dir: &TempDir) -> std::path::PathBuf {
    let file = dir.path().join("form_method.invalid.html");
    std::fs::write(
        &file,
        r#"<!-- Invalid methods -->
<form method="put">Submit</form>
<form method="delete">Submit</form>
<form method="patch">Submit</form>

<!-- Nested in Jinja block -->
{% if show_form %}
    <form method="invalid">Submit</form>
{% endif %}

<!-- Nested elements: inner form inside outer div -->
<div>
    <form method="patch">Submit</form>
</div>
"#,
    )
    .expect("Failed to write fixture file");
    file
}

#[test]
fn check_select_invalid_attr_value_reports_violations() {
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    let output = cli()
        .arg("check")
        .arg("--select=invalid-attr-value")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Invalid value 'put' for attribute 'method'"),
        "expected 'put' violation, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Invalid value 'delete' for attribute 'method'"),
        "expected 'delete' violation, got:\n{stdout}"
    );
}

#[test]
fn check_ignore_invalid_attr_value_suppresses_violations() {
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    assert_cmd_snapshot!(
        cli()
            .arg("check")
            .arg("--ignore=invalid-attr-value")
            .arg(file.as_os_str()),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "#);
}

#[test]
fn check_select_correctness_category_reports_violations() {
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    let output = cli()
        .arg("check")
        .arg("--select=correctness")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid value 'put' for attribute 'method'"));
}

#[test]
fn check_exact_select_beats_category_ignore() {
    // --select=invalid-attr-value (rule level) beats --ignore=correctness
    // (category level) by specificity.
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    let output = cli()
        .arg("check")
        .arg("--select=invalid-attr-value")
        .arg("--ignore=correctness")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid value 'put' for attribute 'method'"));
}

#[test]
fn check_select_all_ignore_specific_disables_rule() {
    // --select=ALL with --ignore=invalid-attr-value: ignore at same Rule
    // specificity wins last-in-order.
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    assert_cmd_snapshot!(
        cli()
            .arg("check")
            .arg("--select=ALL")
            .arg("--ignore=invalid-attr-value")
            .arg(file.as_os_str()),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "#);
}

#[test]
fn check_unknown_selector_errors() {
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    let output = cli()
        .arg("check")
        .arg("--select=nope")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nope"),
        "expected error to mention 'nope', got:\n{stderr}"
    );
}

#[test]
fn check_preview_flag_does_not_error() {
    // No preview rules exist yet; --preview should be a no-op vs default.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<form method=\"post\"></form>\n").unwrap();
    assert_cmd_snapshot!(
        cli().arg("check").arg("--preview").arg(file.as_os_str()),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "#);
}

// ── Rule selection via pyproject.toml ────────────────────────────────

#[test]
fn check_pyproject_ignore_suppresses_violations() {
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    std::fs::write(
        dir.path().join("pyproject.toml"),
        r#"
[tool.djangofmt.lint]
ignore = ["invalid-attr-value"]
"#,
    )
    .unwrap();

    let output = cli()
        .current_dir(dir.path())
        .arg("check")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "expected check to pass with pyproject ignore, got stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn check_cli_select_all_replaces_pyproject_ignore() {
    // pyproject: ignore = ["invalid-attr-value"] -> rule suppressed.
    // CLI: --select=ALL -> the running set is rebuilt from CLI's selectors
    // alone, dropping pyproject's ignore; rule is re-enabled.
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    std::fs::write(
        dir.path().join("pyproject.toml"),
        r#"
[tool.djangofmt.lint]
ignore = ["invalid-attr-value"]
"#,
    )
    .expect("Failed to write pyproject.toml");

    let output = cli()
        .current_dir(dir.path())
        .arg("check")
        .arg("--select=ALL")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected check to fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid value 'put' for attribute 'method'"));
}

#[test]
fn check_cli_select_replaces_pyproject_select() {
    // pyproject: select = [] (nothing enabled)
    // CLI: --select=ALL  -> replacement: invalid-attr-value re-enabled.
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    std::fs::write(
        dir.path().join("pyproject.toml"),
        r"
[tool.djangofmt.lint]
select = []
",
    )
    .unwrap();

    let output = cli()
        .current_dir(dir.path())
        .arg("check")
        .arg("--select=ALL")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected check to fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid value 'put' for attribute 'method'"));
}
