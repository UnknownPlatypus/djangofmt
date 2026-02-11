use djangofmt::{
    commands::format::{FormatterConfig, format_text},
    line_width::{IndentWidth, LineLength},
};
use djangofmt_benchmark::{
    DJANGO_TEMPLATE_DEEPLY_NESTED, DJANGO_TEMPLATE_LARGE, DJANGO_TEMPLATE_SMALL,
    DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS, JINJA_TEMPLATE_LARGE, TestFile,
};

fn main() {
    divan::main();
}

#[divan::bench(args = [
  &DJANGO_TEMPLATE_SMALL,
  &DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS,
  &DJANGO_TEMPLATE_LARGE,
  &DJANGO_TEMPLATE_DEEPLY_NESTED,
  &JINJA_TEMPLATE_LARGE]
)]
fn format_templates(bencher: divan::Bencher, template: &'static TestFile) {
    let config = FormatterConfig::new(LineLength::default(), IndentWidth::default(), None);

    bencher.bench(|| {
        format_text(
            divan::black_box(template.code),
            divan::black_box(&config),
            divan::black_box(template.profile),
        )
        .expect("Formatting to succeed")
    });
}
