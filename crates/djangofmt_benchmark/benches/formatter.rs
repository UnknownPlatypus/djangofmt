use djangofmt_benchmark::{ALL_TEMPLATES, TestFile, warmup};
use djangofmt_formatter::line_width::{IndentWidth, LineLength, SelfClosing};
use djangofmt_formatter::{FormatterConfig, format_text};

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
fn format_templates(bencher: divan::Bencher, template: &'static TestFile) {
    let config = FormatterConfig::new(
        LineLength::default(),
        IndentWidth::default(),
        None,
        vec![],
        SelfClosing::default(),
        false,
    );

    let run = || {
        format_text(
            divan::black_box(template.code),
            divan::black_box(&config),
            divan::black_box(template.profile),
            None,
        )
        .expect("Formatting to succeed")
    };
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(run);
}
