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
    let options = FormatOptions {
        layout: LayoutOptions {
            print_width: 120,
            indent_width: 4,
            ..LayoutOptions::default()
        },
        language: LanguageOptions {
            html_void_self_closing: Some(false),
            svg_self_closing: Some(true),
            mathml_self_closing: Some(true),
            html_normal_self_closing: Some(false),
            prefer_attrs_single_line: false,
            custom_blocks: None,
            ignore_comment_directive: "djangofmt:ignore".into(),
            ignore_file_comment_directive: "djangofmt:ignore".into(),
            ..LanguageOptions::default()
        },
    };

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

fn build_settings(path: &Path) -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path(path.parent().unwrap());
    settings.remove_snapshot_suffix();
    settings.set_prepend_module_to_snapshot(false);
    settings.remove_input_file();
    settings.set_omit_expression(true);
    settings.remove_info();
    settings
}
