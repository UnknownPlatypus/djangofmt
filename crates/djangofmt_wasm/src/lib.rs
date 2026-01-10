use wasm_bindgen::prelude::*;

use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::options::Profile;

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
