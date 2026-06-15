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

const SELECTING_RULES: &str = r#"
By default djangofmt runs every stable rule.

Override that with `select` and `ignore`, either on the command line (`--select`, `--ignore`) or under `[tool.djangofmt.lint]` in `pyproject.toml`.

```toml
[tool.djangofmt.lint]
select = ["category:all"]
ignore = ["category:style", "missing-img-alt"]
preview = true
```

A selector is either:

- single rule name (e.g. `missing-img-alt`)
- a group prefixed with `category:` (e.g. `category:all`, `category:style`, ...)




Preview rules are off by default. Enable them with `--preview` or `preview = true`.

To turn rules off for some files only, map a glob to the selectors to ignore there:

```toml
[tool.djangofmt.lint.per-file-ignores]
"legacy/*" = ["category:accessibility"]
"emails/*.html" = ["missing-img-dimensions", "use-https"]
```

Every pattern is matched both against the file name and against the path relative to the directory holding `pyproject.toml`: `"*.jinja"` covers that extension at any depth, while `"emails/*.html"` is anchored at the project root (and, since `*` crosses `/`, covers nested files under `emails/` too). These are ruff's `per-file-ignores` semantics, so a block can be copied over unchanged.
"#;

fn render() -> String {
    let mut out = String::new();
    out.push_str("---\ntags:\n  - lint\n---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("# Lint rules\n\n");
    out.push_str(SELECTING_RULES);
    for category in RuleCategory::iter() {
        let rules: Vec<Rule> = Rule::iter().filter(|r| r.category() == category).collect();
        if rules.is_empty() {
            continue;
        }
        let _ = writeln!(&mut out, "## {category:?}\n");
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
