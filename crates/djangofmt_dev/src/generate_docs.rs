//! Generate per-rule Markdown documentation under `docs/rules/`.

use std::fmt::Write as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use djangofmt_lint::{FixAvailability, Rule};

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::{REPO_BRANCH, REPO_URL, root_dir};

pub fn main(args: &Args) -> Result<()> {
    for rule in Rule::iter() {
        let path = root_dir()
            .join("docs")
            .join("rules")
            .join(rule.to_string())
            .with_extension("md");
        apply(args.mode, &path, &render(rule))?;
    }
    Ok(())
}

fn render(rule: Rule) -> String {
    let name = rule.to_string();
    let file = rule.source_file().replace('\\', "/");
    let line = rule.source_line();

    let mut output = String::new();
    output.push_str(AUTOGEN_HEADER);
    let _ = writeln!(&mut output, "# {name}");
    let _ = writeln!(&mut output);
    let _ = writeln!(
        &mut output,
        "<small>\n\
         <a href=\"{REPO_URL}/issues?q=sort%3Aupdated-desc%20is%3Aissue%20%22{name}%22\" \
         target=\"_blank\">Related issues</a> ·\n\
         <a href=\"{REPO_URL}/blob/{REPO_BRANCH}/{file}#L{line}\" \
         target=\"_blank\">View source</a>\n\
         </small>\n"
    );

    let fix = rule.fix_availability();
    if matches!(fix, FixAvailability::Always | FixAvailability::Sometimes) {
        let _ = writeln!(&mut output, "{fix}");
        let _ = writeln!(&mut output);
    }

    output.push_str(rule.explanation().trim());
    output.push('\n');
    output
}
