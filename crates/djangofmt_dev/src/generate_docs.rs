//! Generate per-rule Markdown documentation under `docs/rules/`.

use std::fmt::Write as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use djangofmt_lint::{FixAvailability, Rule, RuleGroup};

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::{REPO_BRANCH, REPO_URL, root_dir};

pub fn main(args: &Args) -> Result<()> {
    for rule in Rule::iter() {
        let Some(explanation) = rule.explanation() else {
            // Skip rules with no doc comment: the generator would otherwise
            // emit a near-empty Markdown file.
            continue;
        };
        let path = root_dir()
            .join("docs")
            .join("rules")
            .join(rule.to_string())
            .with_extension("md");
        apply(args.mode, &path, &render(rule, explanation))?;
    }
    Ok(())
}

fn render(rule: Rule, explanation: &str) -> String {
    let name = rule.to_string();
    let file = rule.source_file().replace('\\', "/");
    let line = rule.source_line();

    let mut output = String::new();
    output.push_str(AUTOGEN_HEADER);
    let _ = writeln!(&mut output, "# {name}");
    let _ = writeln!(&mut output);
    let status_text = match rule.group() {
        RuleGroup::Stable { since } => {
            format!(r#"Added in <a href="{REPO_URL}/releases/tag/v{since}">{since}</a>"#)
        }
        RuleGroup::Preview { since } => {
            format!(r#"Preview (since <a href="{REPO_URL}/releases/tag/v{since}">{since}</a>)"#)
        }
        RuleGroup::Deprecated { since } => {
            format!(r#"Deprecated (since <a href="{REPO_URL}/releases/tag/v{since}">{since}</a>)"#)
        }
        RuleGroup::Removed { since } => {
            format!(r#"Removed (since <a href="{REPO_URL}/releases/tag/v{since}">{since}</a>)"#)
        }
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

    output.push_str(explanation.trim());
    output.push('\n');
    output
}
