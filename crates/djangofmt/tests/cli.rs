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
