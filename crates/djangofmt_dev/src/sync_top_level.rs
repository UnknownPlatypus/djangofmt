//! Sync top-level Markdown files (README, CONTRIBUTING) into `docs/` so
//! the Zensical site can render them as first-class pages.
//!
//! Each source file is copied with light textual rewrites: GitHub-flavored
//! alert blockquotes become Material admonitions, top-level README/CONTRIBUTING
//! links are mapped to their docs-site siblings, and CONTRIBUTING references
//! to repo-relative paths (e.g. `./python/...`) are rewritten as absolute
//! GitHub URLs so the Zensical site can build under `--strict`.

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result};

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::{REPO_BRANCH, REPO_URL, root_dir};

pub fn main(args: &Args) -> Result<()> {
    let root = root_dir();

    let readme = fs::read_to_string(root.join("README.md")).context("failed to read README.md")?;
    apply(
        args.mode,
        &root.join("docs").join("index.md"),
        &render_index(&readme),
    )?;

    let contributing = fs::read_to_string(root.join("CONTRIBUTING.md"))
        .context("failed to read CONTRIBUTING.md")?;
    apply(
        args.mode,
        &root.join("docs").join("contributing.md"),
        &render_contributing(&contributing),
    )?;

    Ok(())
}

fn render_index(src: &str) -> String {
    let body = extract_section(src, "Overview")
        .expect("README.md is missing `<!-- Begin section: Overview -->` markers");
    let body = strip_docs_playground_banner(body);
    let body = rewrite_top_level_md_links(&body);
    let body = rewrite_gh_alerts(&body);
    let mut out = String::with_capacity(body.len() + 64);
    // Front matter must be at line 1, so the warning comment comes after.
    // `title: Home` keeps the browser tab from showing "Index" once the
    // README's H1 is excluded by the section markers.
    out.push_str("---\ntitle: Home\n---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str(body.trim_matches('\n'));
    out.push('\n');
    out
}

fn render_contributing(src: &str) -> String {
    let body = rewrite_relative_paths_to_github(src);
    let body = rewrite_gh_alerts(&body);
    let mut out = String::with_capacity(body.len() + AUTOGEN_HEADER.len());
    out.push_str("---\ntags:\n  - contributing\n---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str(&body);
    out
}

/// Extract the slice of `src` between `<!-- Begin section: NAME -->` and
/// `<!-- End section: NAME -->`, exclusive of the markers themselves.
fn extract_section<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let begin = format!("<!-- Begin section: {name} -->");
    let end = format!("<!-- End section: {name} -->");
    let start = src.find(&begin)? + begin.len();
    let stop = src[start..].find(&end)? + start;
    Some(&src[start..stop])
}

/// Drop the README's `**Docs** | **Playground**` banner line — the docs site
/// IS the documentation, so the Docs self-link is redundant there, and the
/// playground link is already exposed through the Material header. The
/// surrounding blank line is collapsed so the rendered page doesn't grow an
/// extra paragraph break.
fn strip_docs_playground_banner(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("[**Docs**]") {
            // Also consume one trailing blank line, if any.
            if lines.peek().is_some_and(|next| next.is_empty()) {
                lines.next();
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Top-level README/CONTRIBUTING references become sibling pages inside
/// the docs site.
fn rewrite_top_level_md_links(src: &str) -> String {
    src.replace("](CONTRIBUTING.md", "](contributing.md")
        .replace("](README.md", "](index.md")
}

/// `](./path)` → `](https://github.com/.../blob/main/path)`. Used by
/// CONTRIBUTING.md, whose `./python/...` links point to files that don't
/// live in `docs/` and would otherwise fail mkdocs' strict link check.
fn rewrite_relative_paths_to_github(src: &str) -> String {
    let prefix = format!("]({REPO_URL}/blob/{REPO_BRANCH}/");
    src.replace("](./", &prefix)
}

/// Rewrite GitHub-flavored alert blockquotes into Material admonitions.
///
/// ```text
/// > [!NOTE]
/// > body line 1
/// > body line 2
/// ```
///
/// becomes:
///
/// ```text
/// !!! note
///
///     body line 1
///     body line 2
/// ```
fn rewrite_gh_alerts(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        let kind = line
            .strip_prefix("> [!")
            .and_then(|rest| rest.strip_suffix(']'))
            .and_then(map_alert_kind);
        let Some(kind) = kind else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let _ = writeln!(&mut out, "!!! {kind}");
        out.push('\n');
        while let Some(next) = lines.peek() {
            if let Some(content) = next.strip_prefix("> ") {
                let _ = writeln!(&mut out, "    {content}");
                lines.next();
            } else if *next == ">" {
                out.push('\n');
                lines.next();
            } else {
                break;
            }
        }
    }
    out
}

fn map_alert_kind(label: &str) -> Option<&'static str> {
    match label {
        "NOTE" => Some("note"),
        "TIP" => Some("tip"),
        "IMPORTANT" => Some("info"),
        "WARNING" => Some("warning"),
        "CAUTION" => Some("danger"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_section_between_markers() {
        let src = "# Title\n\n<!-- Begin section: Overview -->\n\nBody\n\n<!-- End section: Overview -->\n";
        assert_eq!(extract_section(src, "Overview"), Some("\n\nBody\n\n"));
    }

    #[test]
    fn returns_none_for_missing_section() {
        let src = "# Title\n\nBody\n";
        assert_eq!(extract_section(src, "Overview"), None);
    }

    #[test]
    fn strips_docs_playground_banner_with_trailing_blank() {
        let src = "intro\n\n[**Docs**](https://example/docs/) | [**Playground**](https://example/)\n\nrest\n";
        assert_eq!(strip_docs_playground_banner(src), "intro\n\nrest\n");
    }

    #[test]
    fn rewrites_top_level_md_links() {
        assert_eq!(
            rewrite_top_level_md_links("[Contrib](CONTRIBUTING.md)"),
            "[Contrib](contributing.md)"
        );
        assert_eq!(
            rewrite_top_level_md_links("[Home](README.md#install)"),
            "[Home](index.md#install)"
        );
    }

    #[test]
    fn rewrites_gh_alerts() {
        let src = "before\n> [!NOTE]\n> one\n> two\n\nafter\n";
        let expected = "before\n!!! note\n\n    one\n    two\n\nafter\n";
        assert_eq!(rewrite_gh_alerts(src), expected);
    }

    #[test]
    fn preserves_unknown_alert_kinds() {
        let src = "> [!UNKNOWN]\n> body\n";
        assert_eq!(rewrite_gh_alerts(src), "> [!UNKNOWN]\n> body\n");
    }

    #[test]
    fn preserves_plain_blockquotes() {
        let src = "> a quote\n> still quoting\n";
        assert_eq!(rewrite_gh_alerts(src), src);
    }
}
