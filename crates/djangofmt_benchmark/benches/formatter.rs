use djangofmt::{
    commands::format::{FormatterConfig, format_text},
    line_width::{IndentWidth, LineLength, SelfClosing},
};
use djangofmt_benchmark::{ALL_TEMPLATES, TestFile};

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
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
