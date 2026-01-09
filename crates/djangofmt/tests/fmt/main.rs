#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use insta::{assert_snapshot, glob};
use std::{fs, path::Path};

#[test]
fn fmt_snapshot() {
    let pattern = "**/*.html";
    glob!(pattern, |path| {
        let input = fs::read_to_string(path).unwrap();
        let output = run_format_test(path, &input);
        build_settings(path).bind(|| {
            let name = path.file_stem().unwrap().to_str().unwrap();
            assert_snapshot!(name, output);
        });
    });
}

fn run_format_test(path: &Path, input: &str) -> String {
    let config = FormatterConfig::new(120, 4, None);
    let profile = Profile::Django;

    let output = format_text(input, &config, &profile)
        .map_err(|err| format!("failed to format '{}': {:?}", path.display(), err))
        .unwrap()
        .unwrap_or_else(|| input.to_string());
    // Stability test: format the output again and ensure it's the same
    let regression_format = format_text(&output, &config, &profile)
        .map_err(|err| {
            format!(
                "syntax error in stability test '{}': {:?}",
                path.display(),
                err
            )
        })
        .unwrap()
        .unwrap_or_else(|| input.to_string());

    similar_asserts::assert_eq!(
        &output,
        &regression_format,
        "'{}' format is unstable",
        path.display()
    );

    output
}
