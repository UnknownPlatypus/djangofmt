use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

#[path = "../src/test_support.rs"]
mod test_support;
use test_support::Project;

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
    let project = Project::new().file("test.html", "<div   class=\"foo\"  >\n</div>\n");
    assert_cmd_snapshot!(cli().arg(project.join("test.html")), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    1 file reformatted !
    "#);
    assert_eq!(project.read("test.html"), "<div class=\"foo\"></div>\n");
}

#[test]
fn format_already_formatted_file() {
    let project = Project::new().file("test.html", "<div class=\"foo\"></div>\n");
    assert_cmd_snapshot!(cli().arg(project.join("test.html")), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    1 file left unchanged !
    "#);
}

#[test]
fn format_file_with_ignore_directive() {
    let original = "<!-- djangofmt:ignore -->\n<div   class=\"foo\"  ></div>\n";
    let project = Project::new().file("test.html", original);
    assert_cmd_snapshot!(cli().arg(project.join("test.html")), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    1 file skipped !
    "#);
    assert_eq!(project.read("test.html"), original);
}

#[test]
fn format_file_with_jinja_ignore_directive() {
    let original = "{# djangofmt:ignore #}\n<div   class=\"foo\"  ></div>\n";
    let project = Project::new().file("test.html", original);
    assert_cmd_snapshot!(cli().arg(project.join("test.html")), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    1 file skipped !
    "#);
    assert_eq!(project.read("test.html"), original);
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
    let project = Project::new()
        .file("a.html", "<div   ></div>\n")
        .file("b.html", "<span   ></span>\n");
    assert_cmd_snapshot!(cli().arg(project.path()), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    2 files reformatted !
    "#);
}

#[test]
fn format_quiet() {
    let project = Project::new().file("test.html", "<div   ></div>\n");
    assert_cmd_snapshot!(cli().arg("-q").arg(project.join("test.html")), @r#"
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

#[test]
fn format_respects_editorconfig() {
    let project = Project::new()
        .file(
            ".editorconfig",
            "root = true\n\n[*]\nindent_size = 2\nmax_line_length = 40\n",
        )
        .file(
            "test.html",
            "<div class=\"alpha beta gamma delta epsilon\"><span>hello world</span></div>\n",
        );
    assert_cmd_snapshot!(cli().current_dir(project.path()).arg("test.html"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    1 file reformatted !
    "#);
    // The line wraps because of `max_line_length` and is indented by `indent_size`.
    assert_eq!(
        project.read("test.html"),
        "<div class=\"alpha beta gamma delta epsilon\">\n  <span>hello world</span>\n</div>\n"
    );
}

#[test]
fn format_pyproject_overrides_editorconfig() {
    let project = Project::new()
        .file(
            ".editorconfig",
            "root = true\n\n[*]\nindent_size = 2\nmax_line_length = 40\n",
        )
        .file("pyproject.toml", "[tool.djangofmt]\nindent-width = 8\n")
        .file(
            "test.html",
            "<div class=\"alpha beta gamma delta epsilon\"><span>hello world</span></div>\n",
        );
    assert_cmd_snapshot!(cli().current_dir(project.path()).arg("test.html"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    1 file reformatted !
    "#);
    assert_eq!(
        project.read("test.html"),
        "<div class=\"alpha beta gamma delta epsilon\">\n        <span>hello world</span>\n</div>\n"
    );
}

#[test]
fn check_clean_file() {
    let project = Project::new().file("test.html", "<form method=\"post\"></form>\n");
    assert_cmd_snapshot!(cli().arg("check").arg(project.join("test.html")), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    All checks passed!
    "###);
}

#[test]
fn check_file_with_lint_error() {
    let project = Project::new().file("test.html", "<form method=\"put\"></form>\n");
    assert_cmd_snapshot_tmpdir!(cli().arg("check").arg(project.join("test.html")), @r#"
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
    let original = "{% blocktranslate %}Hello{% endblocktranslate %}\n";
    let project = Project::new().file("test.html", original);
    assert_cmd_snapshot_tmpdir!(cli().arg("check").arg(project.join("test.html")), @r#"
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
    assert_eq!(project.read("test.html"), original);
}

#[test]
fn check_fixable_file_with_fix() {
    let project = Project::new().file(
        "test.html",
        "{% blocktranslate %}Hello{% endblocktranslate %}\n",
    );
    assert_cmd_snapshot!(cli().args(["check", "--fix"]).arg(project.join("test.html")), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Found 1 errors (1 fixed, 0 remaining).
    "#);
    // Ensure file was mutated.
    assert_eq!(
        project.read("test.html"),
        "{% blocktranslate trimmed %}Hello{% endblocktranslate %}\n"
    );
}

#[test]
fn check_fixable_file_with_show_fixes() {
    let project = Project::new().file(
        "test.html",
        "{% blocktranslate %}Hello{% endblocktranslate %}\n",
    );
    assert_cmd_snapshot_tmpdir!(
        cli().args(["check", "--fix", "--show-fixes"]).arg(project.join("test.html")),
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
    let project = Project::new().file("test.html", "{% if x %}\n  unclosed\n");
    assert_cmd_snapshot_tmpdir!(cli().args(["check", "--fix"]).arg(project.join("test.html")), @r#"
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
