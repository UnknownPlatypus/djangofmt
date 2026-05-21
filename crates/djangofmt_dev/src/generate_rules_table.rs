//! Generate the rules index page at `docs/rules.md`.

use std::fmt::Write as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use djangofmt_lint::Rule;

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::root_dir;

pub fn main(args: &Args) -> Result<()> {
    let path = root_dir().join("docs").join("rules.md");
    apply(args.mode, &path, &render())
}

fn render() -> String {
    let mut out = String::new();
    out.push_str(AUTOGEN_HEADER);
    out.push_str("# Lint rules\n\n");
    out.push_str(
        "djangofmt's built-in lint rules. Each rule has a stable kebab-case name; \
         click through for the rule's documentation, examples, and source location.\n\n",
    );
    out.push_str("| Name | Category | Fix |\n");
    out.push_str("| ---- | -------- | --- |\n");
    for rule in Rule::iter() {
        let name = rule.to_string();
        let category = rule.category().label();
        let fix = rule.fix_availability().label();
        let _ = writeln!(
            &mut out,
            "| [{name}](rules/{name}.md) | {category} | {fix} |"
        );
    }
    out
}
