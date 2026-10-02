#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt_formatter::line_width::{IndentWidth, LineLength, SelfClosing};
use std::{fs, path};

use djangofmt_formatter::{FormatterConfig, format_text};
use djangofmt_syntax::{ParseError, Profile, graphical_handler};
use insta::{assert_snapshot, glob};
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
    let config = FormatterConfig::new(
        LineLength::default(),
        IndentWidth::default(),
        None,
        SelfClosing::default(),
        false,
    );
    // Use just the filename for display to avoid absolute paths in snapshots
    let display_path = path.file_name().unwrap_or_default().to_string_lossy();

    match format_text(input, &config, Profile::Django, None) {
        Ok(_) => format!("Expected parse error for '{display_path}' but formatting succeeded"),
        Err(err) => render_miette_error(&ParseError::new(&display_path, input.to_string(), &err)),
    }
}

fn render_miette_error(error: &dyn miette::Diagnostic) -> String {
    let mut output = String::new();
    let handler = graphical_handler(GraphicalTheme::unicode_nocolor());
    handler
        .render_report(&mut output, error)
        .expect("Failed to render report");
    output
}
