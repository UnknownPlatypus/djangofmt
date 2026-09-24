//! Generate the enumerated attribute tables at `crates/djangofmt_lint/src/html_spec/generated.rs`
//! from markuplint's HTML spec, downloaded into `target/html-spec/` by `just html-spec`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use crate::generate_all::{Args, apply};
use crate::root_dir;

/// The category of attributes every modern HTML element accepts.
const HTML_GLOBAL_ATTRS: &str = "#HTMLGlobalAttrs";

pub fn main(args: &Args) -> Result<()> {
    let package = root_dir().join("target").join("html-spec").join("package");
    let read = |name: &str| {
        let path = package.join(name);
        fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}, run `just html-spec`", path.display()))
    };
    let spec: Value = serde_json::from_str(&read("index.json")?)?;
    let manifest: Value = serde_json::from_str(&read("package.json")?)?;
    let version = manifest["version"]
        .as_str()
        .context("package.json has no version")?;
    let license = read("LICENSE")?;
    let copyright = license
        .lines()
        .find(|line| line.starts_with("Copyright"))
        .context("LICENSE has no copyright line")?;

    let source = Tables::resolve(&spec)?.render(version, copyright);
    let path = root_dir().join("crates/djangofmt_lint/src/html_spec/generated.rs");
    apply(args.mode, &path, &rustfmt(&source)?)
}

#[derive(Default)]
struct Tables<'a> {
    /// Static name to its `EnumAttr` initializer.
    statics: BTreeMap<String, String>,
    /// Attribute to tag to the static it resolves to, `None` when not an enum there.
    attrs: BTreeMap<&'a str, BTreeMap<&'a str, Option<String>>>,
    /// The static of each enum in `#HTMLGlobalAttrs`.
    globals: BTreeMap<&'a str, String>,
    /// Elements that take every `#HTMLGlobalAttrs` attribute.
    global_tags: BTreeSet<&'a str>,
}

impl<'a> Tables<'a> {
    fn resolve(spec: &'a Value) -> Result<Self> {
        let categories = spec["def"]["#globalAttrs"]
            .as_object()
            .context("no global attributes")?;
        let mut tables = Self::default();
        let html_globals = categories.get(HTML_GLOBAL_ATTRS).and_then(Value::as_object);
        for (name, attr) in html_globals.into_iter().flatten() {
            if let Some(static_name) = tables.enum_static(&name.to_uppercase(), attr)? {
                tables.globals.insert(name, static_name);
            }
        }

        for element in spec["specs"].as_array().context("no element specs")? {
            let tag = element["name"]
                .as_str()
                .context("element spec without a name")?;
            // `svg:*` and `math:*`, which markup_fmt can't tell apart from HTML elements.
            if tag.contains(':') {
                continue;
            }
            let mut attrs = BTreeMap::new();
            for (category, selection) in element["globalAttrs"].as_object().into_iter().flatten() {
                let whole = selection.as_bool() == Some(true);
                if whole && category == HTML_GLOBAL_ATTRS {
                    tables.global_tags.insert(tag);
                }
                let definitions = categories.get(category).and_then(Value::as_object);
                for (name, attr) in definitions.into_iter().flatten() {
                    let listed = selection
                        .as_array()
                        .is_some_and(|names| names.iter().any(|n| n.as_str() == Some(name)));
                    if whole || listed {
                        attrs.insert(
                            name.as_str(),
                            tables.enum_static(&name.to_uppercase(), attr)?,
                        );
                    }
                }
            }
            for (name, attr) in element["attributes"].as_object().into_iter().flatten() {
                // An entry that only documents the attribute keeps the global definition.
                if attr.get("type").is_none() && attr.get("condition").is_none() {
                    continue;
                }
                let static_name = format!("{tag}_{name}").to_uppercase();
                attrs.insert(name.as_str(), tables.enum_static(&static_name, attr)?);
            }
            for (name, resolved) in attrs {
                tables.attrs.entry(name).or_default().insert(tag, resolved);
            }
        }
        Ok(tables)
    }

    /// Register `attr` as a static named `name`, when it is an enum that applies unconditionally.
    fn enum_static(&mut self, name: &str, attr: &Value) -> Result<Option<String>> {
        let ty = &attr["type"];
        let Some(values) = ty["enum"].as_array() else {
            return Ok(None);
        };
        if attr.get("condition").is_some() {
            return Ok(None);
        }
        ensure!(
            ty["disallowToSurroundBySpaces"].as_bool() != Some(false),
            "`{name}` allows surrounding spaces, which `EnumAttr` does not support"
        );
        let values = values
            .iter()
            .map(|value| value.as_str().context("enum value is not a string"))
            .collect::<Result<Vec<_>>>()?;
        let case_insensitive = ty["caseInsensitive"].as_bool().unwrap_or(true);
        let init =
            format!("EnumAttr {{ values: &{values:?}, case_insensitive: {case_insensitive} }}");

        let name = name.replace(['-', ':'], "_");
        match self.statics.get(&name) {
            Some(existing) if *existing != init => bail!("two definitions would be named `{name}`"),
            Some(_) => {}
            None => {
                self.statics.insert(name.clone(), init);
            }
        }
        Ok(Some(name))
    }

    fn render(&self, version: &str, copyright: &str) -> String {
        let mut attrs = String::new();
        let mut used = BTreeSet::new();
        for (&name, tags) in &self.attrs {
            let global = self.globals.get(name);
            // Global elements only list their overrides, the others only their enums.
            let elements: Vec<_> = tags
                .iter()
                .filter(|&(tag, resolved)| {
                    if self.global_tags.contains(tag) {
                        resolved.as_ref() != global
                    } else {
                        resolved.is_some()
                    }
                })
                .map(|(tag, resolved)| {
                    format!("({tag:?}, {})", reference(&mut used, resolved.as_deref()))
                })
                .collect();
            if elements.is_empty() && global.is_none() {
                continue;
            }
            let global = reference(&mut used, global.map(String::as_str));
            let _ = writeln!(
                attrs,
                "Attr {{ name: {name:?}, elements: &[{}], global: {global} }},",
                elements.join(", ")
            );
        }

        let mut output = format!(
            "//! Enumerated attributes from `@markuplint/html-spec` {version}.\n\
             //! Generated by `just html-spec`, do not edit.\n\
             //!\n\
             //! {copyright}, MIT License.\n\n\
             use super::{{Attr, EnumAttr}};\n\n\
             pub static ATTRS: &[Attr] = &[{attrs}];\n\n\
             pub static HTML_GLOBAL_ATTR_TAGS: &[&str] = &{:?};\n\n",
            self.global_tags.iter().collect::<Vec<_>>()
        );
        for name in used {
            let _ = writeln!(output, "static {name}: EnumAttr = {};", self.statics[&name]);
        }
        output
    }
}

/// An `Option<&EnumAttr>` expression pointing at `static_name`, recorded in `used`.
fn reference(used: &mut BTreeSet<String>, static_name: Option<&str>) -> String {
    static_name.map_or_else(
        || "None".to_owned(),
        |name| {
            used.insert(name.to_owned());
            format!("Some(&{name})")
        },
    )
}

fn rustfmt(source: &str) -> Result<String> {
    let mut child = Command::new("rustfmt")
        .current_dir(root_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run rustfmt")?;
    child
        .stdin
        .take()
        .context("rustfmt has no stdin")?
        .write_all(source.as_bytes())?;
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success(),
        "rustfmt rejected the generated source"
    );
    Ok(String::from_utf8(output.stdout)?)
}
