use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::line_width::{IndentWidth, LineLength};
use tracing_test::traced_test;

#[test]
#[traced_test]
fn malva_error_should_log_debug_message() {
    let input = "<style>
.my-class {
color: red;
font-size: 1em;
invalid-css-property;
}
</style>";
    let config = FormatterConfig::new(LineLength::default(), IndentWidth::default(), None);
    let profile = Profile::Django;

    format_text(input, &config, profile).unwrap();
    assert!(logs_contain(
        "Failed to format CSS, falling back to original code"
    ));
    assert!(logs_contain(r#"Unexpected(":", ";")"#));
}
