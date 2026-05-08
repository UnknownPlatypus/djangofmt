use djangofmt::{
    commands::format::{FormatterConfig, format_text},
    line_width::{IndentWidth, LineLength, SelfClosing},
};
use djangofmt_benchmark::{
    DJANGO_TEMPLATE_ATTR_DENSE, DJANGO_TEMPLATE_DEEPLY_NESTED, DJANGO_TEMPLATE_FORM_HEAVY,
    DJANGO_TEMPLATE_LARGE, DJANGO_TEMPLATE_SMALL, DJANGO_TEMPLATE_TAG_DENSE,
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
  &DJANGO_TEMPLATE_TAG_DENSE,
  &DJANGO_TEMPLATE_FORM_HEAVY,
  &DJANGO_TEMPLATE_ATTR_DENSE,
  &JINJA_TEMPLATE_LARGE]
)]
fn format_templates(bencher: divan::Bencher, template: &'static TestFile) {
    let config = FormatterConfig::new(
        LineLength::default(),
        IndentWidth::default(),
        None,
        SelfClosing::default(),
        false,
    );

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(|| {
            format_text(
                divan::black_box(template.code),
                divan::black_box(&config),
                divan::black_box(template.profile),
            )
            .expect("Formatting to succeed")
        });
}
