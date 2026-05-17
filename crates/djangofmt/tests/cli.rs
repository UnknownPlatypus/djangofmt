use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::io::Write;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

fn cli() -> Command {
    Command::new(get_cargo_bin("djangofmt"))
}

/// Spawn the configured command, write `stdin` to it, and capture the output.
fn run_with_stdin(mut cmd: Command, stdin: &str) -> Output {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn djangofmt");
    child
        .stdin
        .take()
        .expect("failed to capture stdin")
        .write_all(stdin.as_bytes())
        .expect("failed to write stdin");
    child
        .wait_with_output()
        .expect("failed to wait for djangofmt")
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

    ----- stderr -----
    1 file reformatted !
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

    ----- stderr -----
    1 file left unchanged !
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

    ----- stderr -----
    1 file skipped !
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

    ----- stderr -----
    2 files reformatted !
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

// ── Format from stdin ────────────────────────────────────────────────

#[test]
fn format_stdin_dash_sentinel() {
    assert_cmd_snapshot!(
        cli().arg("-").pass_stdin("<div   class=\"foo\"  ></div>\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <div class="foo"></div>

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_already_formatted() {
    assert_cmd_snapshot!(
        cli().arg("-").pass_stdin("<div class=\"foo\"></div>\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <div class="foo"></div>

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_with_filename_html() {
    assert_cmd_snapshot!(
        cli()
            .args(["--stdin-filename", "foo.html"])
            .pass_stdin("<div   class=\"foo\"  ></div>\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <div class="foo"></div>

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_with_filename_infers_jinja_profile() {
    // `{% if x %}...{% endif %}` is valid in both profiles, so use the `raw` block
    // which is jinja-specific: jinja preserves its inner content while django does not.
    assert_cmd_snapshot!(
        cli()
            .args(["--stdin-filename", "foo.jinja"])
            .pass_stdin("{% if x %}hi{% endif %}\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    {% if x %}hi{% endif %}

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_ignore_directive() {
    let source = "<!-- djangofmt:ignore -->\n<div   class=\"foo\"  ></div>\n";
    assert_cmd_snapshot!(
        cli().arg("-").pass_stdin(source),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <!-- djangofmt:ignore -->
    <div   class="foo"  ></div>

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_parse_error_exits_2() {
    let mut cmd = cli();
    cmd.arg("-");
    let output = run_with_stdin(cmd, "<div   class=\"foo\"  >");
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("expected close tag for opening tag <div>"),
        "stderr was: {stderr}"
    );
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
}

#[test]
fn format_stdin_force_exclude_parrots_input() {
    let source = "<div   class=\"foo\"  ></div>\n";
    assert_cmd_snapshot!(
        cli()
            .args([
                "--force-exclude",
                "--extend-exclude",
                "foo.html",
                "--stdin-filename",
                "foo.html",
            ])
            .pass_stdin(source),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <div   class="foo"  ></div>

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_extra_file_warns_but_uses_stdin() {
    // When --stdin-filename is set, any other file path is ignored with a warning.
    assert_cmd_snapshot!(
        cli()
            .args(["--stdin-filename", "stream.html"])
            .arg("on_disk.html")
            .pass_stdin("<div   class=\"foo\"  ></div>\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <div class="foo"></div>

    ----- stderr -----
    Ignoring file on_disk.html in favor of standard input.
    "#);
}

#[test]
fn format_stdin_filename_alone_without_dash() {
    // --stdin-filename should make `files` optional (mirrors ruff).
    assert_cmd_snapshot!(
        cli()
            .args(["--stdin-filename", "foo.html"])
            .pass_stdin("<div   class=\"foo\"  ></div>\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    <div class="foo"></div>

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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid value 'put' for attribute 'method'"));
    assert!(stderr.contains("Use one of: get, post, dialog"));
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
