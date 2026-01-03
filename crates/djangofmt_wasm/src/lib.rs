use wasm_bindgen::prelude::*;

use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme};
use serde::Serialize;

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
        .map_err(into_error)
}

#[derive(Serialize)]
struct LintResult {
    error_count: usize,
    output: String,
}
#[wasm_bindgen]
pub fn lint(source: &str) -> Result<JsValue, JsError> {
    let mut parser = Parser::new(source, markup_fmt::Language::Jinja, vec![]);
    let ast = match parser.parse_root() {
        Ok(ast) => ast,
        Err(e) => {
            let result = LintResult {
                error_count: 1,
                output: format!("Parse error: {e:?}"),
            };
            return to_js_value(&result);
        }
    };

    let settings = Settings::default();
    let diagnostics = check_ast(&ast, &settings);
    let error_count = diagnostics.len();

    if error_count == 0 {
        let result = LintResult {
            error_count: 0,
            output: String::new(),
        };
        return to_js_value(&result);
    }

    let file_diagnostics = FileDiagnostics::new(source.to_string(), diagnostics);
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut output = String::new();
    handler
        .render_report(&mut output, &file_diagnostics)
        .map_err(into_error)?;

    let result = LintResult {
        error_count,
        output,
    };
    to_js_value(&result)
}

pub(crate) fn into_error<E: std::fmt::Display>(err: E) -> JsError {
    JsError::new(&err.to_string())
}

fn to_js_value<T: Serialize>(data: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(data).map_err(into_error)
}
