use insta::{Settings, assert_snapshot, glob};
use markup_fmt::{
    Language,
    config::{FormatOptions, LanguageOptions, LayoutOptions},
    format_text,
};
use std::{borrow::Cow, fs, path::Path};

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

fn run_parse_error_test(path: &Path, input: &str) -> String {
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

    let result = format_text(input, Language::Django, &options, |code, _| {
        Ok::<_, ()>(Cow::from(code))
    });

    match result {
        Ok(_) => format!(
            "Expected parse error for '{}' but formatting succeeded",
            path.display()
        ),
        Err(err) => {
            // Format the error as it would appear with miette
            format!("{err:?}")
        }
    }
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
