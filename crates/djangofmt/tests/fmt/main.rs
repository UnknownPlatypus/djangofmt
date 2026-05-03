#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::line_width::{IndentWidth, LineLength, SelfClosing};
use insta::{assert_snapshot, glob};
use std::{fs, path::Path};

#[test]
fn fmt_snapshot() {
    let pattern = "**/*.html";
    glob!(pattern, |path| {
        // Skip files covered by fmt_snapshot_preserve_unquoted
        if path.components().any(|c| c.as_os_str() == "preserve_unquoted") {
            return;
        }
        let input = fs::read_to_string(path).unwrap();
        let output = run_format_test(path, &input, false);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

#[test]
fn fmt_snapshot_preserve_unquoted() {
    glob!("preserve_unquoted/**/*.html", |path| {
        let input = fs::read_to_string(path).unwrap();
        let output = run_format_test(path, &input, true);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

fn run_format_test(path: &Path, input: &str, preserve_unquoted_attrs: bool) -> String {
    let config = FormatterConfig::new(
        LineLength::default(),
        IndentWidth::default(),
        None,
        SelfClosing::default(),
        preserve_unquoted_attrs,
    );
    let profile = Profile::Django;

    let output = format_text(input, &config, profile)
        .map_err(|err| format!("failed to format '{}': {:?}", path.display(), err))
        .expect("Failed to format text in test")
        .unwrap_or_else(|| input.to_string());
    // Stability test: format the output again and ensure it's the same
    let regression_format = format_text(&output, &config, profile)
        .map_err(|err| {
            format!(
                "syntax error in stability test '{}': {:?}",
                path.display(),
                err
            )
        })
        .expect("Failed to format text in stability test")
        .unwrap_or_else(|| input.to_string());

    similar_asserts::assert_eq!(
        &output,
        &regression_format,
        "'{}' format is unstable",
        path.display()
    );

    output
}
