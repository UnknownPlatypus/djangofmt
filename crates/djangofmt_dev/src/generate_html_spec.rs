//! Generate the enumerated attribute tables at `crates/djangofmt_lint/src/html_spec/generated.rs`
//! from markuplint's HTML spec, downloaded into `target/html-spec/` by `just html-spec`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, ensure};
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
    apply(args.mode, &path, &source)
}

#[derive(Default)]
struct Tables<'a> {
    /// Static name to its keywords.
    statics: BTreeMap<String, Vec<&'a str>>,
    /// Attribute to tag to the static holding its keywords on that element.
    attrs: BTreeMap<&'a str, BTreeMap<&'a str, String>>,
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
            if let Some(keywords) = keywords(attr)? {
                let static_name = tables.register(name, &keywords)?;
                tables.globals.insert(name, static_name);
                tables.attrs.entry(name).or_default();
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
            // Attribute to the name its static would take and its keywords, if an enum.
            let mut element_attrs = BTreeMap::new();
            for (category, selection) in element["globalAttrs"].as_object().into_iter().flatten() {
                let whole_category = selection.as_bool() == Some(true);
                if whole_category && category == HTML_GLOBAL_ATTRS {
                    tables.global_tags.insert(tag);
                    continue;
                }
                let definitions = categories.get(category).and_then(Value::as_object);
                for (name, attr) in definitions.into_iter().flatten() {
                    let listed = selection
                        .as_array()
                        .is_some_and(|names| names.iter().any(|n| n.as_str() == Some(name)));
                    if whole_category || listed {
                        element_attrs.insert(name.as_str(), (name.clone(), keywords(attr)?));
                    }
                }
            }
            for (name, attr) in element["attributes"].as_object().into_iter().flatten() {
                // An entry that only documents the attribute keeps the global definition.
                if attr.get("type").is_none() && attr.get("condition").is_none() {
                    continue;
                }
                element_attrs.insert(name.as_str(), (format!("{tag}_{name}"), keywords(attr)?));
            }
            for (name, (static_name, keywords)) in element_attrs {
                let Some(keywords) = keywords else {
                    ensure!(
                        !tables.global_tags.contains(tag) || !tables.globals.contains_key(name),
                        "`<{tag} {name}>` replaces a global enum with another type"
                    );
                    continue;
                };
                let static_name = tables.register(&static_name, &keywords)?;
                tables
                    .attrs
                    .entry(name)
                    .or_default()
                    .insert(tag, static_name);
            }
        }
        Ok(tables)
    }

    /// Name the static holding `keywords`, failing when another definition already took the name.
    fn register(&mut self, name: &str, keywords: &[&'a str]) -> Result<String> {
        let name = name.to_ascii_uppercase().replace(['-', ':'], "_");
        if let Some(previous) = self.statics.insert(name.clone(), keywords.to_vec()) {
            ensure!(
                previous == keywords,
                "two definitions would be named `{name}`"
            );
        }
        Ok(name)
    }

    fn render(&self, version: &str, copyright: &str) -> String {
        let mut rows = String::new();
        for (name, tags) in &self.attrs {
            let elements: Vec<_> = tags
                .iter()
                .map(|(tag, static_name)| format!("({tag:?}, {static_name})"))
                .collect();
            let global = self.globals.get(name).map_or_else(
                || "None".to_owned(),
                |static_name| format!("Some({static_name})"),
            );
            let _ = writeln!(
                rows,
                "Attr {{ name: {name:?}, elements: &[{}], global: {global} }},",
                elements.join(", ")
            );
        }

        let mut output = format!(
            "//! Enumerated attributes from `@markuplint/html-spec` {version}.\n\
             //! Generated by `just html-spec`, do not edit.\n\
             //!\n\
             //! {copyright}, MIT License.\n\n\
             use super::Attr;\n\n\
             pub static ATTRS: &[Attr] = &[{rows}];\n\n\
             pub static HTML_GLOBAL_ATTR_TAGS: &[&str] = &{:?};\n\n",
            self.global_tags.iter().collect::<Vec<_>>()
        );
        for (name, keywords) in &self.statics {
            let _ = writeln!(output, "static {name}: &[&str] = &{keywords:?};");
        }
        output
    }
}

/// The keywords of `attr`, when it is an enum that applies unconditionally.
fn keywords(attr: &Value) -> Result<Option<Vec<&str>>> {
    let ty = &attr["type"];
    let Some(values) = ty["enum"].as_array() else {
        return Ok(None);
    };
    if attr.get("condition").is_some() {
        return Ok(None);
    }
    let values = values
        .iter()
        .map(|value| value.as_str().context("enum value is not a string"))
        .collect::<Result<Vec<_>>>()?;
    // The lint matches keywords untrimmed and ASCII case-insensitively,
    // which a case-sensitive enum only agrees with when it lists every case, like `<ol type>`.
    ensure!(
        ty["disallowToSurroundBySpaces"].as_bool() != Some(false),
        "{values:?} allows surrounding spaces"
    );
    ensure!(
        ty["caseInsensitive"].as_bool() != Some(false)
            || values.iter().all(|value| {
                values.contains(&value.to_ascii_lowercase().as_str())
                    && values.contains(&value.to_ascii_uppercase().as_str())
            }),
        "{values:?} is case-sensitive"
    );
    Ok(Some(values))
}
