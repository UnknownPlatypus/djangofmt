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
impl LintResult {
    const fn new(error_count: usize, output: String) -> Self {
        Self {
            error_count,
            output,
        }
    }
}
#[wasm_bindgen]
pub fn lint(source: &str) -> Result<JsValue, JsError> {
    lint_inner(source).and_then(|result| serde_wasm_bindgen::to_value(&result).map_err(into_error))
}

fn lint_inner(source: &str) -> Result<LintResult, JsError> {
    let mut parser = Parser::new(source, markup_fmt::Language::Jinja, vec![]);
    let ast = match parser.parse_root() {
        Ok(ast) => ast,
        Err(e) => {
            let result = LintResult::new(1, format!("Parse error: {e:?}"));
            return Ok(result);
        }
    };

    let settings = Settings::default();
    let diagnostics = check_ast(&ast, &settings);
    let error_count = diagnostics.len();

    if error_count == 0 {
        let result = LintResult::new(0, String::new());
        return Ok(result);
    }

    let file_diagnostics = FileDiagnostics::new(source.to_string(), diagnostics);
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut output = String::new();
    handler
        .render_report(&mut output, &file_diagnostics)
        .map_err(into_error)?;

    // Remove the `× Found x lint error(s)` header that is redundant
    // with error_count in the playground.
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
