#![allow(clippy::unwrap_used)]
#[path = "../common.rs"]
mod common;

use common::build_settings;
use djangofmt::commands::format::build_markup_options;
use insta::{Settings, assert_snapshot, glob};
use markup_fmt::{
    Language,
    config::{FormatOptions, LanguageOptions, LayoutOptions},
    format_text,
};
use std::{borrow::Cow, fs, path::Path};

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
    let options = build_markup_options(120, 4, None);

    let output = format_text(input, Language::Django, &options, |code, _| {
        Ok::<_, ()>(Cow::from(code))
    })
    .map_err(|err| format!("failed to format '{}': {:?}", path.display(), err))
    .unwrap();

    // Stability test: format the output again and ensure it's the same
    let regression_format = format_text(&output, Language::Django, &options, |code, _| {
        Ok::<_, ()>(Cow::from(code))
    })
    .map_err(|err| {
        format!(
            "syntax error in stability test '{}': {:?}",
            path.display(),
            err
        )
    })
    .unwrap();

    similar_asserts::assert_eq!(
        output,
        regression_format,
        "'{}' format is unstable",
        path.display()
    );

    output
}
