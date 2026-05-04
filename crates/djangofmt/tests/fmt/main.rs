#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::line_width::{IndentWidth, LineLength, SelfClosing};
use insta::{assert_snapshot, glob};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

/// Test-only config struct matching the options exposed by djangofmt.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestFormatOptions {
    #[serde(default)]
    preserve_unquoted_attrs: bool,
}

#[test]
fn fmt_snapshot() {
    glob!("**/*.html", |path| {
        let input = fs::read_to_string(path).unwrap();

        let options = fs::read_to_string(path.with_file_name("config.toml"))
            .map(|config_file| {
                toml::from_str::<HashMap<String, TestFormatOptions>>(&config_file).unwrap()
            })
            .ok();

        if let Some(options) = options {
            options.into_iter().for_each(|(option_name, options)| {
                let output =
                    run_format_test(path, &input, options.preserve_unquoted_attrs);
                build_settings(path).bind(|| {
                    let name = path.file_stem().unwrap().to_str().unwrap();
                    assert_snapshot!(format!("{name}.{option_name}"), output);
                });
            })
        } else {
            let output = run_format_test(path, &input, false);
            build_settings(path).bind(|| {
                let name = path.file_stem().unwrap().to_str().unwrap();
                assert_snapshot!(name, output);
            });
        }
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
