#![expect(
    clippy::result_large_err,
    reason = "test surfaces the large error type directly"
)]
#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt_formatter::line_width::{IndentWidth, LineLength, SelfClosing};
use std::{fs, path};

use djangofmt_formatter::build_markup_options;
use djangofmt_syntax::ParseError;
use djangofmt_syntax::Profile;
use djangofmt_syntax::graphical_handler;
use insta::{assert_snapshot, glob};
use markup_fmt::config::FormatOptions;
use markup_fmt::{Language, format_text};
use miette::GraphicalTheme;

#[test]
fn parse_error_snapshot() {
    let pattern = "**/*.html";
    glob!(pattern, |path| {
        let input = fs::read_to_string(path).unwrap();
        let error_output = run_parse_error_test(path, &input);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, error_output);
        });
    });
}

fn run_parse_error_test(path: &path::Path, input: &str) -> String {
    let options = build_markup_options(
        LineLength::default(),
        IndentWidth::default(),
        None,
        vec![],
        SelfClosing::default(),
        false,
    );
    // Use just the filename for display to avoid absolute paths in snapshots
    let display_path = path.file_name().unwrap_or_default().to_string_lossy();

    match format_str(input, &display_path, &options, Profile::Django) {
        Ok(_) => format!("Expected parse error for '{display_path}' but formatting succeeded"),
        Err(err) => render_miette_error(&err),
    }
}

fn format_str(
    input: &str,
    name: &str,
    format_options: &FormatOptions,
    profile: Profile,
) -> Result<String, ParseError> {
    let format_result = format_text(input, Language::from(profile), format_options, |code, _| {
        Ok(code.into())
    });

    format_result.map_err(|err| ParseError::new(name, input.to_string(), &err))
}

fn render_miette_error(error: &dyn miette::Diagnostic) -> String {
    let mut output = String::new();
    let handler = graphical_handler(GraphicalTheme::unicode_nocolor());
    handler
        .render_report(&mut output, error)
        .expect("Failed to render report");
    output
}
