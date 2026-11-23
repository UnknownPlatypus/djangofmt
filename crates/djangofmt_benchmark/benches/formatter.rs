use djangofmt_benchmark::{profile, warmup};
use djangofmt_corpus::{ALL_TEMPLATES, Template};
use djangofmt_formatter::line_width::{IndentWidth, LineLength, SelfClosing};
use djangofmt_formatter::{FormatterConfig, format_text};

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
fn format_templates(bencher: divan::Bencher, template: &'static Template) {
    let config = FormatterConfig::new(
        LineLength::default(),
        IndentWidth::default(),
        None,
        SelfClosing::default(),
        false,
    );

    let profile = profile(template);
    let run = || {
        format_text(
            divan::black_box(template.code),
            divan::black_box(&config),
            divan::black_box(profile),
            None,
        )
        .expect("Formatting to succeed")
    };
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(run);
}
