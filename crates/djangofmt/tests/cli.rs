use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;
use tempfile::TempDir;

fn cli() -> Command {
    Command::new(get_cargo_bin("djangofmt"))
}

/// Like [`assert_cmd_snapshot!`] but redacts the leading directory of `test.html`
/// occurrences (i.e. the per-run `TempDir` prefix in miette diagnostics).
macro_rules! assert_cmd_snapshot_tmpdir {
    ($cmd:expr, @$snapshot:literal $(,)?) => {
        insta::with_settings!(
            { filters => vec![(r"[^\s\[]+/test\.html", "[TMP]/test.html")] },
            { assert_cmd_snapshot!($cmd, @$snapshot) }
        )
    };
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
fn format_file_with_jinja_ignore_directive() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    let original = "{# djangofmt:ignore #}\n<div   class=\"foo\"  ></div>\n";
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
    // Jinja whitespace-control modifiers (`{{- ... -}}`) are preserved under the jinja
    // profile, but stripped under django — proving the profile was inferred from `.jinja`.
    assert_cmd_snapshot!(
        cli()
            .args(["--stdin-filename", "foo.jinja"])
            .pass_stdin("{{- foo -}}\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    {{- foo -}}

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
fn format_stdin_jinja_ignore_directive() {
    let source = "{# djangofmt:ignore #}\n<div   class=\"foo\"  ></div>\n";
    assert_cmd_snapshot!(
        cli().arg("-").pass_stdin(source),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    {# djangofmt:ignore #}
    <div   class="foo"  ></div>

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_parse_error_exits_2() {
    assert_cmd_snapshot!(
        cli().arg("-").pass_stdin("<div   class=\"foo\"  >"),
        @r#"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
      × expected close tag for opening tag <div>
       ╭─[<unknown>:1:1]
     1 │ <div   class="foo"  >
       · ─┬─
       ·  ╰── here
       ╰────
    "#);
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
    // --stdin-filename should make `files` optional.
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
    assert_cmd_snapshot!(cli().arg("check").arg(file.as_os_str()), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    All checks passed!
    "###);
}

#[test]
fn check_file_with_lint_error() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "<form method=\"put\"></form>\n").unwrap();
    assert_cmd_snapshot_tmpdir!(cli().arg("check").arg(file.as_os_str()), @r#"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × Found 1 lint error(s)
      ╰─▶   × Invalid value 'put' for attribute 'method'.
             ╭─[[TMP]/test.html:1:15]
           1 │ <form method="put"></form>
             ·               ─┬─
             ·                ╰── here
             ╰────
            help: Use one of: get, post, dialog

    Found 1 errors.
    "#);
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
    assert_cmd_snapshot_tmpdir!(cli().arg("check").arg(file.as_os_str()), @r#"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × Found 1 lint error(s)
      ╰─▶   × `{% blocktranslate %}` should declare `trimmed` to avoid leaking
            │ indentation into translation strings.
             ╭─[[TMP]/test.html:1:3]
           1 │ {% blocktranslate %}Hello{% endblocktranslate %}
             ·   ────────┬───────
             ·           ╰── here
             ╰────
            help: Add `trimmed` to the opening tag, e.g. `{% blocktranslate
                  trimmed %}...{% endblocktranslate %}`.

    Found 1 errors. [*] 1 fixable with the --fix option.
    "#);
    // Ensure we didn't apply anything without --fix.
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
}

#[test]
fn check_fixable_file_with_fix() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "{% blocktranslate %}Hello{% endblocktranslate %}\n").unwrap();
    assert_cmd_snapshot!(cli().args(["check", "--fix"]).arg(file.as_os_str()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Found 1 errors (1 fixed, 0 remaining).
    "#);
    // Ensure file was mutated.
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "{% blocktranslate trimmed %}Hello{% endblocktranslate %}\n"
    );
}

#[test]
fn check_fixable_file_with_show_fixes() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "{% blocktranslate %}Hello{% endblocktranslate %}\n").unwrap();
    assert_cmd_snapshot_tmpdir!(
        cli().args(["check", "--fix", "--show-fixes"]).arg(file.as_os_str()),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Found 1 errors (1 fixed, 0 remaining).
    Fixed 1 errors:
    - [TMP]/test.html:
        1 × untrimmed-blocktranslate (Add trimmed)
    "#);
}

#[test]
fn check_malformed_file_with_fix_surfaces_parse_error() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.html");
    std::fs::write(&file, "{% if x %}\n  unclosed\n").unwrap();
    assert_cmd_snapshot_tmpdir!(cli().args(["check", "--fix"]).arg(file.as_os_str()), @r#"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × unclosed {% if %} block.
       ╭─[[TMP]/test.html:1:4]
     1 │ {% if x %}
       ·    ─┬
       ·     ╰── here
     2 │   unclosed
       ╰────
      help: Check for invalid HTML syntax inside the block that might prevent
            finding the end tag.

    Couldn't check 1 files!
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid value 'put' for attribute 'method'"),
        "expected 'put' violation, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Invalid value 'delete' for attribute 'method'"),
        "expected 'delete' violation, got:\n{stderr}"
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
    All checks passed!
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid value 'put' for attribute 'method'"));
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid value 'put' for attribute 'method'"));
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
    All checks passed!
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
    // `--preview` enables preview rules; on a clean file it stays quiet.
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
    All checks passed!
    "#);
}

#[test]
fn check_preview_rule_gated_by_preview_flag() {
    // `empty-tag-pair` is a preview rule. Exact-selecting it must NOT enable
    // it without `--preview` (preview rules require preview mode); with
    // `--preview` it runs and flags the empty pair.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("empty.html");
    std::fs::write(&file, "<span></span>\n").unwrap();

    let without = cli()
        .arg("check")
        .arg("--select=empty-tag-pair")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        without.status.success(),
        "preview rule should stay off without --preview, got stderr:\n{}",
        String::from_utf8_lossy(&without.stderr),
    );

    let with = cli()
        .arg("check")
        .arg("--select=empty-tag-pair")
        .arg("--preview")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        !with.status.success(),
        "preview rule should run with --preview"
    );
    let stderr = String::from_utf8_lossy(&with.stderr);
    assert!(
        stderr.contains("Empty `<span>` tag pair"),
        "expected empty-tag-pair violation, got:\n{stderr}"
    );
}

#[test]
fn check_extend_select_invalid_attr_value_reports_violations() {
    // `--extend-select` adds on top of the default rule set; the violation is
    // reported (verifies the flag is wired through to the resolver).
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    let output = cli()
        .arg("check")
        .arg("--extend-select=invalid-attr-value")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid value 'put' for attribute 'method'"),
        "expected 'put' violation, got:\n{stderr}"
    );
}

#[test]
fn check_extend_ignore_invalid_attr_value_suppresses_violations() {
    // `--extend-ignore` subtracts from the resolved set; the default
    // `invalid-attr-value` rule is removed and the check passes.
    let dir = TempDir::new().unwrap();
    let file = write_invalid_form_method_file(&dir);
    assert_cmd_snapshot!(
        cli()
            .arg("check")
            .arg("--extend-ignore=invalid-attr-value")
            .arg(file.as_os_str()),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    All checks passed!
    "#);
}

#[test]
fn check_no_preview_overrides_pyproject_preview() {
    // `empty-tag-pair` is a preview rule. pyproject enables preview and selects
    // it; CLI `--no-preview` must win (CLI > pyproject), so the rule does not
    // run. This exercises the `--preview`/`--no-preview` `overrides_with` flag.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("empty.html");
    std::fs::write(&file, "<span></span>\n").unwrap();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        r#"
[tool.djangofmt.lint]
select = ["empty-tag-pair"]
preview = true
"#,
    )
    .unwrap();

    // Sanity: with pyproject `preview = true` the preview rule runs.
    let with = cli()
        .current_dir(dir.path())
        .arg("check")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        !with.status.success(),
        "preview rule should run with pyproject preview = true, got stderr:\n{}",
        String::from_utf8_lossy(&with.stderr),
    );

    // `--no-preview` overrides pyproject and turns preview off.
    let without = cli()
        .current_dir(dir.path())
        .arg("check")
        .arg("--no-preview")
        .arg(file.as_os_str())
        .output()
        .unwrap();
    assert!(
        without.status.success(),
        "`--no-preview` must override pyproject preview = true, got stderr:\n{}",
        String::from_utf8_lossy(&without.stderr),
    );
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid value 'put' for attribute 'method'"));
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid value 'put' for attribute 'method'"));
}
