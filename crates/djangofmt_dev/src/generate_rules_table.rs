//! Generate the rules index page at `docs/rules.md`.

use std::fmt::Write as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use djangofmt_lint::{FixAvailability, Rule, RuleCategory, RuleGroup};

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::root_dir;

pub fn main(args: &Args) -> Result<()> {
    let path = root_dir().join("docs").join("rules.md");
    apply(args.mode, &path, &render())
}

const INTRO: &str = include_str!("../docs/lint-rules-intro.md");

// Same legend as ruff's rules table.
const FIX_SYMBOL: &str = "🛠️";
const PREVIEW_SYMBOL: &str = "🧪";
const REMOVED_SYMBOL: &str = "❌";
const WARNING_SYMBOL: &str = "⚠️";
const SPACER: &str = "&nbsp;&nbsp;&nbsp;&nbsp;";

fn render() -> String {
    let mut out = String::new();
    out.push_str("---\ntags:\n  - lint\n---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str(INTRO);
    out.push_str("\n## Legend\n\n");
    let _ = writeln!(
        &mut out,
        "{SPACER}{PREVIEW_SYMBOL}{SPACER} The rule is unstable and is in preview.<br />\n\
         {SPACER}{WARNING_SYMBOL}{SPACER} The rule has been deprecated and will be removed in a future release.<br />\n\
         {SPACER}{REMOVED_SYMBOL}{SPACER} The rule has been removed; only the documentation is available.<br />\n\
         {SPACER}{FIX_SYMBOL}{SPACER} The rule is automatically fixable by the `--fix` command-line option.\n"
    );
    for category in RuleCategory::iter() {
        let rules: Vec<Rule> = Rule::iter().filter(|r| r.category() == category).collect();
        if rules.is_empty() {
            continue;
        }
        let _ = writeln!(&mut out, "## {category:?}\n");
        out.push_str("| Name | Message | |\n");
        out.push_str("| ---- | ------- | -: |\n");
        for rule in rules {
            let name = rule.to_string();
            let status = match rule.group() {
                RuleGroup::Stable { .. } => String::new(),
                RuleGroup::Preview { since } => {
                    symbol(PREVIEW_SYMBOL, &format!("In preview since {since}"))
                }
                RuleGroup::Deprecated { since } => {
                    symbol(WARNING_SYMBOL, &format!("Deprecated since {since}"))
                }
                RuleGroup::Removed { since } => {
                    symbol(REMOVED_SYMBOL, &format!("Removed in {since}"))
                }
            };
            let fix = match rule.fix_availability() {
                FixAvailability::Always | FixAvailability::Sometimes => {
                    symbol(FIX_SYMBOL, "Automatic fix available")
                }
                FixAvailability::None => String::new(),
            };
            let _ = writeln!(
                &mut out,
                "| [{name}](rules/{name}.md) | {} | {status} {fix} |",
                message(rule)
            );
        }
        out.push('\n');
    }
    out
}

fn symbol(symbol: &str, title: &str) -> String {
    format!("<span title='{title}'>{symbol}</span>")
}

/// The rule's first message format, as the user would read it.
fn message(rule: Rule) -> String {
    let message = rule.message_formats().first().copied().unwrap_or_default();
    // The macro captures the raw `format!` string, so undo its brace doubling.
    let message = message.replace("{{", "{").replace("}}", "}");
    // A trailing `{x}` placeholder trips zensical's attr_list parser by being read as HTML
    // attributes. Render it as `{x\}` so the closing brace is escaped.
    message
        .strip_suffix('}')
        .map_or_else(|| message.clone(), |prefix| format!("{prefix}\\}}"))
}
