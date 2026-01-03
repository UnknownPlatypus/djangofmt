use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// Format Django or Jinja markup according to the given line length, indent width, and profile.
///
/// The function returns the formatted source text; when the formatter makes no changes the original
/// source is returned unchanged. Use "jinja" for Jinja profile, any other value selects the Django profile.
///
/// # Examples
///
/// ```
/// let src = "{% if true %}  hello  {% endif %}";
/// let out = format(src, 80, 4, "django").unwrap();
/// assert!(!out.is_empty());
/// ```
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

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
struct LintResult {
    error_count: usize,
    output: String,
}
impl LintResult {
    /// Create a new LintResult with the given error count and formatted output.
    ///
    /// # Examples
    ///
    /// ```
    /// let res = LintResult::new(2, "first\nsecond".to_string());
    /// assert_eq!(res.error_count, 2);
    /// assert_eq!(res.output, "first\nsecond");
    /// ```
    const fn new(error_count: usize, output: String) -> Self {
        Self {
            error_count,
            output,
        }
    }
}
/// Lint markup source and return structured diagnostics for wasm consumers.
///
/// # Parameters
///
/// - `source`: Markup text (Jinja/Django template) to lint.
///
/// # Returns
///
/// A `JsValue` containing a serialized `LintResult` with `error_count` and `output` on success; a `JsError` if linting or serialization fails.
///
/// # Examples
///
/// ```
/// // In wasm-hosted Rust tests this demonstrates basic usage; in JS the exported
/// // function yields the same serialized result.
/// let result = djangofmt_wasm::lint("{{ invalid }}").unwrap();
/// // `result` is a JsValue holding the serialized LintResult
/// ```
#[wasm_bindgen]
pub fn lint(source: &str) -> Result<JsValue, JsError> {
    lint_inner(source).and_then(|result| serde_wasm_bindgen::to_value(&result).map_err(into_error))
}

/// Lints Jinja source text and returns a structured summary of lint findings.
///
/// If parsing fails, the function returns a `LintResult` with `error_count` set to 1
/// and `output` containing a debug-formatted parse error message starting with
/// `"Parse error: "`. If no lint diagnostics are produced, `error_count` will be 0
/// and `output` will be an empty string. Otherwise, `output` contains a graphical
/// diagnostic report (the leading `"× Found x lint error(s)"` header is removed).
///
/// # Examples
///
/// ```
/// // No errors
/// let ok = lint_inner("plain text without lints").unwrap();
/// assert_eq!(ok.error_count, 0);
/// assert!(ok.output.is_empty());
///
/// // Parse error yields error_count == 1 and an error message
/// let parsed = lint_inner("{% invalid").unwrap();
/// assert_eq!(parsed.error_count, 1);
/// assert!(parsed.output.starts_with("Parse error:"));
/// ```
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

/// Convert a Display-able error or value into a wasm `JsError` using its string representation.
///
/// This creates a `JsError` whose message is the result of `err.to_string()`.
///
/// # Examples
///
/// ```
/// let js_err = crate::into_error("something went wrong");
/// // js_err can be returned to JavaScript with the original message
/// ```
pub(crate) fn into_error<E: std::fmt::Display>(err: E) -> JsError {
    JsError::new(&err.to_string())
}
