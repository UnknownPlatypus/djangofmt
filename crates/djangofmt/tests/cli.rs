use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

#[path = "../src/test_support.rs"]
mod test_support;
use test_support::Project;

fn cli() -> Command {
    Command::new(get_cargo_bin("djangofmt"))
}

/// Like [`assert_cmd_snapshot!`] but redacts the leading directory of `.html` paths
/// (i.e. the per-run `TempDir` prefix in miette diagnostics).
macro_rules! assert_cmd_snapshot_tmpdir {
    ($cmd:expr, @$snapshot:literal $(,)?) => {
        insta::with_settings!(
            { filters => vec![(r"[^\s\[]+/(\w+\.html)", "[TMP]/$1")] },
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
fn check_unparsable_file_with_ignore_directive() {
    let project = Project::new().file("test.html", "{# djangofmt:   ignore #}\n<div>\n");
    assert_cmd_snapshot!(cli().arg("check").arg(project.join("test.html")), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    All checks passed!
    ");
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

#[test]
fn format_file_parse_error_exits_2() {
    let project = Project::new().file("test.html", "<div   class=\"foo\"  >");
    assert_cmd_snapshot_tmpdir!(cli().arg(project.join("test.html")), @r##"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----

      × expected close tag for opening tag <div>
       ╭─[[TMP]/test.html:1:2]
     1 │ <div   class="foo"  >
       ·  ─┬─
       ·   ╰── here
       ╰────
      help: If a `</div>` does exist, it must live in the same block as the
            opening tag.
            https://unknownplatypus.github.io/djangofmt/docs/known-limitations/#conditional-openclose-tags

    Couldn't format 1 files!
    "##);
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
fn format_stdin_pyproject_profile_beats_extension() {
    // Same precedence as the file path (CLI > pyproject > extension): the configured
    // django profile wins over `.jinja` and keeps `{% verbatim %}` content raw.
    let project = Project::new().file("pyproject.toml", "[tool.djangofmt]\nprofile = \"django\"\n");
    assert_cmd_snapshot!(
        cli()
            .current_dir(project.path())
            .args(["--stdin-filename", "foo.jinja"])
            .pass_stdin("{% verbatim %}{{   x   }}{% endverbatim %}\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    {% verbatim %}{{   x   }}{% endverbatim %}

    ----- stderr -----
    "#);
}

#[test]
fn format_stdin_with_filename_infers_jinja_profile() {
    // `verbatim` is not a raw block for jinja, so its content gets formatted — while the
    // django profile keeps it untouched — proving the profile was inferred from `.jinja`.
    assert_cmd_snapshot!(
        cli()
            .args(["--stdin-filename", "foo.jinja"])
            .pass_stdin("{% verbatim %}{{   x   }}{% endverbatim %}\n"),
        @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    {% verbatim %}{{ x }}{% endverbatim %}

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
    assert_cmd_snapshot!(
        cli().arg("-").pass_stdin("<div   class=\"foo\"  >"),
        @r##"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----

      × expected close tag for opening tag <div>
       ╭─[<unknown>:1:2]
     1 │ <div   class="foo"  >
       ·  ─┬─
       ·   ╰── here
       ╰────
      help: If a `</div>` does exist, it must live in the same block as the
            opening tag.
            https://unknownplatypus.github.io/djangofmt/docs/known-limitations/#conditional-openclose-tags
    "##);
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
    assert_cmd_snapshot_tmpdir!(cli().arg("check").arg(project.join("test.html")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × Invalid value 'put' for attribute 'method'.
       ╭─[[TMP]/test.html:1:15]
     1 │ <form method="put"></form>
       ·               ─┬─
       ·                ╰── here
       ╰────
      help: Use one of: get, post, dialog

    Found 1 errors.
    "###);
}

#[test]
fn check_concise_output_format() {
    let project = Project::new()
        .file(
            "test.html",
            "<form method=\"put\"></form>\n{% blocktranslate %}Hello{% endblocktranslate %}\n",
        )
        .file("unparsable.html", "<div>\n");
    assert_cmd_snapshot_tmpdir!(
        cli().args(["check", "--output-format", "concise"]).arg(project.path()),
        @r###"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    [TMP]/unparsable.html:1:2: expected close tag for opening tag <div>
    Couldn't check 1 files!
    [TMP]/test.html:1:15: invalid-attr-value Invalid value 'put' for attribute 'method'.
    [TMP]/test.html:2:3: untrimmed-blocktranslate [*] `{% blocktranslate %}` should declare `trimmed` to avoid leaking indentation into translation strings.
    Found 2 errors. [*] 1 fixable with the --fix option.
    "###);
}

#[test]
fn check_fixable_file_without_fix() {
    let original = "{% blocktranslate %}Hello{% endblocktranslate %}\n";
    let project = Project::new().file("test.html", original);
    assert_cmd_snapshot_tmpdir!(cli().arg("check").arg(project.join("test.html")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × `{% blocktranslate %}` should declare `trimmed` to avoid leaking
      │ indentation into translation strings.
       ╭─[[TMP]/test.html:1:3]
     1 │ {% blocktranslate %}Hello{% endblocktranslate %}
       ·   ────────┬───────
       ·           ╰── here
       ╰────
      help: Add `trimmed` to the opening tag, e.g. `{% blocktranslate trimmed
            %}...{% endblocktranslate %}`.

    Found 1 errors. [*] 1 fixable with the --fix option.
    "###);
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
fn check_passes_file_path_to_path_aware_rules() {
    // `same-file-partial-include` only fires when the checked file's path reaches the rule:
    // cover both the report path (`lint_source`) and the fix path (`lint_fix`).
    let project = Project::new().file(
        "app/page.html",
        "{% partialdef nav %}<a>Home</a>{% endpartialdef %}\n{% include \"app/page.html#nav\" %}\n",
    );
    let args = [
        "check",
        "--preview",
        "--select",
        "same-file-partial-include",
    ];
    assert_cmd_snapshot_tmpdir!(
        cli().args(args).args(["--output-format", "concise"]).arg(project.join("app/page.html")),
        @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    [TMP]/page.html:2:1: same-file-partial-include [*] Same-file partial `nav` rendered via `{% include %}`.
    Found 1 errors. [*] 1 fixable with the --fix option.
    "###
    );
    assert_cmd_snapshot!(cli().args(args).arg("--fix").arg(project.join("app/page.html")), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Found 1 errors (1 fixed, 0 remaining).
    "###);
    assert_eq!(
        project.read("app/page.html"),
        "{% partialdef nav %}<a>Home</a>{% endpartialdef %}\n{% partial nav %}\n"
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
    assert_cmd_snapshot_tmpdir!(cli().args(["check", "--fix"]).arg(project.join("test.html")), @r###"
    success: false
    exit_code: 2
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
    "###);
}

#[test]
fn check_respects_pyproject_custom_blocks() {
    // `check` must parse with the same custom blocks as `format`: without them
    // this unclosed `{% stage %}` is two flat tags and silently passes.
    let project = Project::new()
        .file(
            "pyproject.toml",
            "[tool.djangofmt]\ncustom-blocks = [\"stage\"]\n",
        )
        .file("test.html", "{% stage %}\n<p>hi</p>\n");
    assert_cmd_snapshot!(cli().current_dir(project.path()).args(["check", "test.html"]), @r###"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----

      × unclosed {% stage %} block.
       ╭─[test.html:1:4]
     1 │ {% stage %}
       ·    ──┬──
       ·      ╰── here
     2 │ <p>hi</p>
       ╰────
      help: Check for invalid HTML syntax inside the block that might prevent
            finding the end tag.

    Couldn't check 1 files!
    "###);
}

#[test]
fn check_respects_pyproject_per_file_ignores() {
    // Same violation in both files: the glob must silence it in `legacy/` only.
    let violation = "<form method=\"put\"></form>\n";
    let project = Project::new()
        .file(
            "pyproject.toml",
            "[tool.djangofmt.lint.per-file-ignores]\n\"legacy/*\" = [\"invalid-attr-value\"]\n",
        )
        .file("legacy/old.html", violation)
        .file("new.html", violation);
    assert_cmd_snapshot!(cli().current_dir(project.path()).args(["check", "."]), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × Invalid value 'put' for attribute 'method'.
       ╭─[new.html:1:15]
     1 │ <form method="put"></form>
       ·               ─┬─
       ·                ╰── here
       ╰────
      help: Use one of: get, post, dialog

    Found 1 errors.
    "###);

    // Globs anchor at the `pyproject.toml` directory, not the cwd: `legacy/*` keeps
    // matching when djangofmt runs from inside `legacy/`.
    assert_cmd_snapshot_tmpdir!(cli().current_dir(project.join("legacy")).args(["check", ".."]), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
      × Invalid value 'put' for attribute 'method'.
       ╭─[[TMP]/new.html:1:15]
     1 │ <form method="put"></form>
       ·               ─┬─
       ·                ╰── here
       ╰────
      help: Use one of: get, post, dialog

    Found 1 errors.
    "###);
}
