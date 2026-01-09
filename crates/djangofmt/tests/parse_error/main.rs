#![allow(clippy::result_large_err)]
#[path = "../common.rs"]
mod common;

use common::build_settings;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::{fs, path};

use djangofmt::args::Profile;
use djangofmt::commands::format::{ParseError, build_markup_options};
use insta::{assert_snapshot, glob};
use markup_fmt::config::FormatOptions;
use markup_fmt::{Language, format_text};
use miette::{GraphicalReportHandler, GraphicalTheme};

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
    let options = build_markup_options(120, 4, None);
    // Use just the filename for display to avoid absolute paths in snapshots
    let display_path = path.file_name().map(Path::new).map(Path::to_path_buf);

    match format_str(input, display_path, &options, &Profile::Django) {
        Ok(_) => format!(
            "Expected parse error for '{}' but formatting succeeded",
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
        Err(err) => render_miette_error(&err),
    }
}

fn format_str(
    input: &str,
    name: Option<PathBuf>,
    format_options: &FormatOptions,
    profile: &Profile,
) -> Result<String, ParseError> {
    let format_result = format_text(input, Language::from(profile), format_options, |code, _| {
        Ok::<_, ()>(Cow::from(code))
    });

    format_result.map_err(|err| ParseError::new(name, input.to_string(), &err))
}

fn render_miette_error(error: &dyn miette::Diagnostic) -> String {
    let mut output = String::new();
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    handler.render_report(&mut output, error).unwrap();
    output
}
