//! Generate the settings reference at `docs/settings.md` from the `pyproject.toml` options
//! metadata.

use std::fmt::Write as _;

use anyhow::Result;

use djangofmt::options_metadata::{OptionField, OptionSet, OptionsMetadata, Visit};
use djangofmt::pyproject::PyprojectSettings;

use crate::generate_all::{AUTOGEN_HEADER, Args, apply};
use crate::root_dir;

pub fn main(args: &Args) -> Result<()> {
    let path = root_dir().join("docs").join("settings.md");
    apply(args.mode, &path, &render())
}

fn render() -> String {
    let mut output = String::new();
    output.push_str(AUTOGEN_HEADER);
    output.push_str(
        "# Settings\n\n\
         djangofmt reads its configuration from the `[tool.djangofmt]` table of the closest \
         `pyproject.toml`; command-line arguments take precedence over it.\n\n",
    );
    render_set(&mut output, "", PyprojectSettings::metadata());
    output
}

/// Render a table of options, `path` being the dotted path it sits at under `[tool.djangofmt]`
/// (empty for the top-level table).
fn render_set(output: &mut String, path: &str, set: OptionSet) {
    if path.is_empty() {
        output.push_str("## Top-level\n\n");
    } else {
        let _ = writeln!(output, "## `{path}`\n");
    }
    if let Some(documentation) = set.documentation() {
        let _ = writeln!(output, "{documentation}\n");
    }

    let mut collector = Collector::default();
    set.record(&mut collector);

    for (name, field) in &collector.fields {
        render_field(output, path, name, field);
    }
    for (name, sub_set) in collector.sets {
        render_set(output, &join(path, &name), sub_set);
    }
}

fn render_field(output: &mut String, path: &str, name: &str, field: &OptionField) {
    let anchor = join(path, name).replace('.', "_");
    let table = join(
        "tool.djangofmt",
        &join(path, field.scope.unwrap_or_default()),
    );
    let _ = writeln!(output, "### [`{name}`](#{anchor}) {{: #{anchor} }}\n");
    let _ = writeln!(output, "{}\n", field.doc);
    let _ = writeln!(output, "**Default value**: `{}`\n", field.default);
    let _ = writeln!(output, "**Type**: `{}`\n", field.value_type);
    let _ = writeln!(
        output,
        "**Example usage**:\n\n```toml\n[{table}]\n{}\n```\n",
        field.example
    );
    output.push_str("---\n\n");
}

/// Join two dotted path segments, either of which may be empty.
fn join(prefix: &str, suffix: &str) -> String {
    match (prefix, suffix) {
        ("", suffix) => suffix.to_string(),
        (prefix, "") => prefix.to_string(),
        (prefix, suffix) => format!("{prefix}.{suffix}"),
    }
}

#[derive(Default)]
struct Collector {
    fields: Vec<(String, OptionField)>,
    sets: Vec<(String, OptionSet)>,
}

impl Visit for Collector {
    fn record_field(&mut self, name: &str, field: OptionField) {
        self.fields.push((name.to_string(), field));
    }

    fn record_set(&mut self, name: &str, set: OptionSet) {
        self.sets.push((name.to_string(), set));
    }
}
