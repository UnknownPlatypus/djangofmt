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
    All checks passed!

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

#[test]
fn check_fixable_file_without_fix() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    let original = "{% blocktranslate %}Hello{% endblocktranslate %}\n";
    std::fs::write(&file, original).unwrap();
    let output = cli().arg("check").arg(file.as_os_str()).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Found 1 errors. [*] 1 fixable with the --fix option."),
        "summary missing or wrong, got:\n{stdout}"
    );
    // Ensure we didn't apply anything without --fix.
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, original);
}

#[test]
fn check_fixable_file_with_fix() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    let original = "{% blocktranslate %}Hello{% endblocktranslate %}\n";
    std::fs::write(&file, original).unwrap();
    let output = cli()
        .args(["check", "--fix"])
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "expected success exit, got {output:?}"
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Found 1 errors (1 fixed, 0 remaining)."),
        "summary missing or wrong, got:\n{stdout}"
    );
    // Ensure file was mutated.
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        after,
        "{% blocktranslate trimmed %}Hello{% endblocktranslate %}\n"
    );
}

#[test]
fn check_fixable_file_with_show_fixes() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    let original = "{% blocktranslate %}Hello{% endblocktranslate %}\n";
    std::fs::write(&file, original).unwrap();
    let output = cli()
        .args(["check", "--fix", "--show-fixes"])
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "expected success exit, got {output:?}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Found 1 errors (1 fixed, 0 remaining)."),
        "summary missing or wrong, got:\n{stdout}"
    );
    assert!(stdout.contains("Fixed 1 errors:"), "got:\n{stdout}");
    // Per-rule line must include rule code AND fix_title.
    assert!(
        stdout.contains("1 × untrimmed-blocktranslate (Add trimmed)"),
        "per-rule line missing fix_title, got:\n{stdout}"
    );
}

#[test]
fn check_malformed_file_with_fix_surfaces_parse_error() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "{% if x %}\n  unclosed\n").unwrap();
    let output = cli()
        .args(["check", "--fix"])
        .arg(file.as_os_str())
        .output()
        .unwrap();
    // Parse errors must surface even with --fix.
    assert!(
        !output.status.success(),
        "expected failure exit, got {output:?}"
    );
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("unclosed {% if %} block"),
        "expected parse error rendering, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Couldn't check 1 files!"),
        "expected error count line, got:\n{stdout}"
    );
    // Must NOT claim success.
    assert!(
        !stdout.contains("All checks passed!"),
        "must not claim success when parse errors occurred, got:\n{stdout}"
    );
}
