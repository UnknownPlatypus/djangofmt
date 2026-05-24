//! Generate the rules index page at `docs/rules.md`.

use std::fmt::Write as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use djangofmt_lint::{Rule, RuleCategory};

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::root_dir;

pub fn main(args: &Args) -> Result<()> {
    let path = root_dir().join("docs").join("rules.md");
    apply(args.mode, &path, &render())
}

fn render() -> String {
    let mut out = String::new();
    out.push_str("---\ntags:\n  - lint\n---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("# Lint rules\n\n");
    out.push_str(
        "djangofmt's built-in lint rules, grouped by category. Each rule has a stable \
         kebab-case name; click through for the rule's documentation, examples, and \
         source location.\n\n",
    );
    for category in RuleCategory::iter() {
        let rules: Vec<Rule> = Rule::iter().filter(|r| r.category() == category).collect();
        if rules.is_empty() {
            continue;
        }
        let _ = writeln!(&mut out, "## {}\n", category.label());
        out.push_str("| Name | Message | Fix |\n");
        out.push_str("| ---- | ------- | --- |\n");
        for rule in rules {
            let name = rule.to_string();
            let fix = rule.fix_availability().label();
            let message = rule.message_formats().first().copied().unwrap_or_default();
            // `{x}` placeholders in the format string trip zensical attr_list parser by being read as HTML attributes.
            // Render them as `{x\}` so the closing brace is escaped.
            let message = message
                .strip_suffix('}')
                .map_or_else(|| message.to_string(), |prefix| format!("{prefix}\\}}"));
            let _ = writeln!(
                &mut out,
                "| [{name}](rules/{name}.md) | {message} | {fix} |"
            );
        }
        out.push('\n');
    }
    out
}
