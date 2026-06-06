//! Generate per-rule Markdown documentation under `docs/rules/`.

use std::fmt::Write as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use djangofmt_lint::{FixAvailability, Rule, RuleGroup};

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::{REPO_BRANCH, REPO_URL, root_dir};

/// Substituted with the next release version by release tooling. Until then
/// we render it as plain text rather than building a broken `tag/v…` URL.
const NEXT_VERSION_PLACEHOLDER: &str = "NEXT_DJANGOFMT_VERSION";

pub fn main(args: &Args) -> Result<()> {
    let dir = root_dir().join("docs").join("rules");
    for rule in Rule::iter() {
        let Some(explanation) = rule.explanation() else {
            // Skip rules with no doc comment: the generator would otherwise
            // emit a near-empty Markdown file.
            continue;
        };
        let path = dir.join(rule.to_string()).with_extension("md");
        apply(args.mode, &path, &render(rule, explanation))?;
    }
    Ok(())
}

/// Render the "Added in / Preview / Deprecated / Removed (since X)" header
/// fragment. Wrap `since` in a release-tag link when it's a real version,
/// or leave it as plain text for the `NEXT_DJANGOFMT_VERSION` placeholder.
fn since_link(since: &str) -> String {
    if since == NEXT_VERSION_PLACEHOLDER {
        since.to_string()
    } else {
        format!(r#"<a href="{REPO_URL}/releases/tag/v{since}">{since}</a>"#)
    }
}

fn render(rule: Rule, explanation: &str) -> String {
    let name = rule.to_string();
    let file = rule.source_file().replace('\\', "/");
    let line = rule.source_line();
    let category = rule.category();

    let mut output = String::new();
    let _ = writeln!(&mut output, "---\ntags:\n  - lint - {category}\n---\n");
    output.push_str(AUTOGEN_HEADER);
    let _ = writeln!(&mut output, "# {name}");
    let _ = writeln!(&mut output);
    let status_text = match rule.group() {
        RuleGroup::Stable { since } => format!("Added in {}", since_link(since)),
        RuleGroup::Preview { since } => format!("Preview (since {})", since_link(since)),
        RuleGroup::Deprecated { since } => format!("Deprecated (since {})", since_link(since)),
        RuleGroup::Removed { since } => format!("Removed (since {})", since_link(since)),
    };
    let _ = writeln!(
        &mut output,
        "<small>\n\
         {status_text} ·\n\
         <a href=\"{REPO_URL}/issues?q=sort%3Aupdated-desc%20is%3Aissue%20%22{name}%22\" \
         target=\"_blank\">Related issues</a> ·\n\
         <a href=\"{REPO_URL}/blob/{REPO_BRANCH}/{file}#L{line}\" \
         target=\"_blank\">View source</a>\n\
         </small>\n"
    );

    if rule.is_deprecated() {
        output.push_str(
            "**Warning: This rule is deprecated and will be removed in a future release.**\n\n",
        );
    }
    if rule.is_removed() {
        output.push_str(
            "**Warning: This rule has been removed and its documentation is only available for historical reasons.**\n\n",
        );
    }

    let fix = rule.fix_availability();
    if matches!(fix, FixAvailability::Always | FixAvailability::Sometimes) {
        let _ = writeln!(&mut output, "{fix}");
        let _ = writeln!(&mut output);
    }

    if rule.is_preview() {
        output.push_str(
            "This rule is unstable and in preview. The `--preview` flag is required for use.\n\n",
        );
    }

    output.push_str(ensure_blank_after_headings(explanation.trim()).trim_end());
    output.push('\n');
    output
}

/// Insert a blank line after any ATX heading (`#`, `##`, …) that is immediately
/// followed by non-empty content, so the rendered Markdown matches the usual
/// convention. Lines inside fenced code blocks are left alone.
fn ensure_blank_after_headings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code_fence = false;
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        out.push_str(line);
        out.push('\n');
        if line.trim_start().starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if !in_code_fence
            && line.starts_with('#')
            && lines.peek().is_some_and(|next| !next.is_empty())
        {
            out.push('\n');
        }
    }
    out
}
