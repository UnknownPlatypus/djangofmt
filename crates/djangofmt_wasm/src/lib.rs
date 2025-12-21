use wasm_bindgen::prelude::*;

use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt_lint::check_ast;
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme};

#[wasm_bindgen]
pub fn format(
    source: &str,
    line_length: usize,
    indent_width: usize,
    profile: &str,
) -> Result<String, JsError> {
    let profile = match profile {
        "jinja" => Profile::Jinja,
        _ => Profile::Django,
    };
    let config = FormatterConfig::new(line_length, indent_width, None);

    format_text(source, &config, &profile)
        .map(|opt| opt.unwrap_or_else(|| source.to_string()))
        .map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn lint(source: &str) -> String {
    let mut parser = Parser::new(source, markup_fmt::Language::Jinja, vec![]);
    let ast = match parser.parse_root() {
        Ok(ast) => ast,
        Err(e) => return format!("Parse error: {e:?}"),
    };

    let diagnostics = check_ast(source, &ast);

    if diagnostics.is_empty() {
        return String::new();
    }

    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let mut output = String::new();

    for diag in &diagnostics {
        let _ = handler.render_report(&mut output, diag);
    }

    output
}
