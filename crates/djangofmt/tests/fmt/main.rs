#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::pyproject::PyprojectSettings;
use insta::{assert_snapshot, glob};
use std::{collections::HashMap, fs, path::Path};

#[test]
fn fmt_snapshot() {
    glob!("**/*.html", |path| {
        let input = fs::read_to_string(path).unwrap();

        let options = fs::read_to_string(path.with_file_name("config.toml"))
            .map(|config_file| {
                toml::from_str::<HashMap<String, PyprojectSettings>>(&config_file).unwrap()
            })
            .ok();

        if let Some(options) = options {
            options.into_iter().for_each(|(option_name, pyproject)| {
                let config = build_config(&pyproject);
                let output = run_format_test(path, &input, &config);
                build_settings(path).bind(|| {
                    let name = path.file_stem().unwrap().to_str().unwrap();
                    assert_snapshot!(format!("{name}.{option_name}"), output);
                });
            })
        } else {
            let config = build_config(&PyprojectSettings::default());
            let output = run_format_test(path, &input, &config);
            build_settings(path).bind(|| {
                let name = path.file_stem().unwrap().to_str().unwrap();
                assert_snapshot!(name, output);
            });
        }
    });
}

fn build_config(pyproject: &PyprojectSettings) -> FormatterConfig {
    FormatterConfig::new(
        pyproject.line_length.unwrap_or_default(),
        pyproject.indent_width.unwrap_or_default(),
        pyproject.custom_blocks.clone(),
        pyproject.html_void_self_closing.unwrap_or_default(),
        pyproject.preserve_unquoted_attrs.unwrap_or_default(),
    )
}

fn run_format_test(path: &Path, input: &str, config: &FormatterConfig) -> String {
    let profile = Profile::Django;

    let output = format_text(input, config, profile)
        .map_err(|err| format!("failed to format '{}': {:?}", path.display(), err))
        .expect("Failed to format text in test")
        .unwrap_or_else(|| input.to_string());
    // Stability test: format the output again and ensure it's the same
    let regression_format = format_text(&output, config, profile)
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
