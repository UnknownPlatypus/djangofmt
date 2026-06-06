use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::error::ParseError;
use djangofmt::line_width::{IndentWidth, LineLength, SelfClosing};
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource};
use serde::Serialize;
use std::path::PathBuf;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use web_sys::console;
#[wasm_bindgen]
#[must_use]
pub fn format(source: &str, line_length: u16, indent_width: u8, profile: &str) -> String {
    let profile = get_profile(profile);
    let line_length = LineLength::try_from(line_length).unwrap_or_default();
    let indent_width = IndentWidth::try_from(indent_width).unwrap_or_default();
    let config = FormatterConfig::new(
        line_length,
        indent_width,
        None,
        SelfClosing::default(),
        false,
    );

    match format_text(source, &config, profile) {
        Ok(opt) => opt.unwrap_or_else(|| source.to_string()),
        Err(err) => render_parse_error(source, &err),
    }
}

#[wasm_bindgen]
#[must_use]
pub fn ast(source: &str, profile: &str) -> String {
    let profile = get_profile(profile);
    let mut parser = Parser::new(source, markup_fmt::Language::from(profile), vec![]);
    match parser.parse_root() {
        Ok(ast) => format!("{ast:#?}"),
        Err(e) => {
            let err: markup_fmt::FormatError = markup_fmt::FormatError::Syntax(e);
            render_parse_error(source, &err)
        }
    }
}

#[wasm_bindgen]
#[must_use]
pub fn doc_tree(source: &str, line_length: u16, indent_width: u8, profile: &str) -> String {
    let profile = get_profile(profile);
    let line_length = LineLength::try_from(line_length).unwrap_or_default();
    let indent_width = IndentWidth::try_from(indent_width).unwrap_or_default();
    let config = FormatterConfig::new(
        line_length,
        indent_width,
        None,
        SelfClosing::default(),
        false,
    );

    markup_fmt::debug_doc_tree(
        source,
        markup_fmt::Language::from(profile),
        &config.markup,
        |code, _| Ok(code.into()),
    )
    .map_or_else(
        |err| render_parse_error(source, &err),
        |ir| compact_doc_ir(&ir),
    )
}

struct DocNode {
    label: String,
    children: Vec<Self>,
}

// `markup_fmt` only exposes the Doc IR via `Debug` (`tiny_pretty::Nest` is not
// nameable outside its crate), so reparse that output into a compact tree.
fn compact_doc_ir(debug: &str) -> String {
    let lines: Vec<&str> = debug.lines().collect();
    let mut pos = 0;
    let mut out = String::new();
    for node in parse_doc_nodes(&lines, &mut pos) {
        render_doc_node(&node, 0, &mut out);
    }
    out
}

fn parse_doc_nodes(lines: &[&str], pos: &mut usize) -> Vec<DocNode> {
    let mut nodes = Vec::new();
    while let Some(line) = lines.get(*pos) {
        let trimmed = line.trim();
        let token = trimmed.strip_suffix(',').unwrap_or(trimmed);
        if token == ")" || token == "]" {
            *pos += 1;
            break;
        }
        *pos += 1;
        if let Some(name) = token.strip_suffix('(') {
            let children = parse_doc_nodes(lines, pos);
            nodes.push(DocNode {
                label: name.to_string(),
                children,
            });
        } else if token == "[" {
            let children = parse_doc_nodes(lines, pos);
            nodes.push(DocNode {
                label: "[".to_string(),
                children,
            });
        } else {
            nodes.push(DocNode {
                label: token.to_string(),
                children: Vec::new(),
            });
        }
    }
    nodes
}

fn render_doc_node(node: &DocNode, level: usize, out: &mut String) {
    use std::fmt::Write;

    let prefix = "|   ".repeat(level);
    match node.label.as_str() {
        "List" | "Group" | "Slice" => {
            let _ = writeln!(out, "{prefix}{level}---{}", node.label);
            for child in array_elems(node) {
                render_doc_node(child, level + 1, out);
            }
        }
        "Nest" => {
            let width = node.children.first().map_or("", |n| n.label.as_str());
            let _ = writeln!(out, "{prefix}{level}---Nest({width})");
            if let Some(wrapper) = node.children.get(1) {
                for child in nest_children(wrapper) {
                    render_doc_node(child, level + 1, out);
                }
            }
        }
        "Text" | "Char" => {
            let arg = node.children.first().map_or("", |n| n.label.as_str());
            let _ = writeln!(out, "{prefix}{level}---{}({arg})", node.label);
        }
        "Break" => {
            let args: Vec<&str> = node.children.iter().map(|n| n.label.as_str()).collect();
            let _ = writeln!(out, "{prefix}{level}---Break({})", args.join(", "));
        }
        _ => {
            let _ = writeln!(out, "{prefix}{level}---{}", node.label);
            for child in &node.children {
                render_doc_node(child, level + 1, out);
            }
        }
    }
}

fn array_elems(node: &DocNode) -> &[DocNode] {
    match node.children.first() {
        Some(c) if c.label == "[" => &c.children,
        Some(c) if c.label == "[]" => &[],
        _ => &node.children,
    }
}

fn nest_children(wrapper: &DocNode) -> &[DocNode] {
    match wrapper.label.as_str() {
        "Vec" | "Slice" => array_elems(wrapper),
        _ => &wrapper.children,
    }
}

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
struct LintResult {
    error_count: usize,
    output: String,
}
impl LintResult {
    const fn new(error_count: usize, output: String) -> Self {
        Self {
            error_count,
            output,
        }
    }
}
#[wasm_bindgen]
pub fn lint(source: &str, profile: &str) -> Result<JsValue, JsError> {
    lint_inner(source, profile)
        .and_then(|result| serde_wasm_bindgen::to_value(&result).map_err(into_error))
}

fn lint_inner(source: &str, profile: &str) -> Result<LintResult, JsError> {
    let profile = get_profile(profile);
    let mut parser = Parser::new(source, markup_fmt::Language::from(profile), vec![]);
    let ast = match parser.parse_root() {
        Ok(ast) => ast,
        Err(e) => {
            let err: markup_fmt::FormatError = markup_fmt::FormatError::Syntax(e);
            return Ok(LintResult::new(1, render_parse_error(source, &err)));
        }
    };

    let settings = Settings::all();
    let diagnostics = check_ast(source, &ast, &settings);
    let error_count = diagnostics.len();

    if error_count == 0 {
        let result = LintResult::new(0, String::new());
        return Ok(result);
    }

    let file_diagnostics =
        FileDiagnostics::new(NamedSource::new("", source.to_string()), diagnostics);
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut output = String::new();
    handler
        .render_report(&mut output, &file_diagnostics)
        .map_err(into_error)?;

    // Drop the redundant 2-line `× Found N lint error(s)` header.
    let skip_pos = output
        .char_indices()
        .filter(|(_, c)| *c == '\n')
        .nth(1)
        .map_or(0, |(i, _)| i + 1);
    output.drain(..skip_pos);

    let result = LintResult::new(error_count, output);
    Ok(result)
}

pub(crate) fn into_error<E: std::fmt::Display>(err: E) -> JsError {
    JsError::new(&err.to_string())
}

fn get_profile(profile: &str) -> Profile {
    match profile.to_ascii_lowercase().as_str() {
        "jinja" => Profile::Jinja,
        "django" => Profile::Django,
        _ => {
            console::log_1(
                &format!("Invalid profile: '{profile}'. Falling back to 'Django'").into(),
            );
            Profile::Django
        }
    }
}

fn render_parse_error(source: &str, err: &markup_fmt::FormatError) -> String {
    let diagnostic = ParseError::new(
        Some(PathBuf::from("template.html")),
        source.to_string(),
        err,
    );
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let mut output = String::new();
    match handler.render_report(&mut output, &diagnostic) {
        Ok(()) => output,
        Err(err) => err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn control_flow_and_element() {
        let src = r#"{% if a %}<div class="x">hi</div>{% endif %}"#;
        insta::assert_snapshot!(super::doc_tree(src, 80, 4, "django"));
    }

    #[test]
    fn nested_elements_with_attributes() {
        let src = r#"<ul class="list"><li data-id="1">one</li><li data-id="2">two</li></ul>"#;
        insta::assert_snapshot!(super::doc_tree(src, 80, 4, "django"));
    }
}
