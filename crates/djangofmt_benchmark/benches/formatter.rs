use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt_benchmark::{LARGE_DJANGO_TEMPLATE, LARGE_JINJA_TEMPLATE, NESTED_DJANGO_TEMPLATE};

fn main() {
    divan::main();
}

#[divan::bench(name = "Large jinja template (2.7k LoC)")]
fn large_jinja_template(bencher: divan::Bencher) {
    let config = FormatterConfig::new(120, 4, None);

    bencher.bench_local(|| {
        format_text(
            divan::black_box(LARGE_JINJA_TEMPLATE.code),
            divan::black_box(&config),
            divan::black_box(&LARGE_JINJA_TEMPLATE.profile),
        )
        .expect("Formatting to succeed")
    });
}

#[divan::bench(name = "Large Django template (1.3k LoC)")]
fn large_django_template(bencher: divan::Bencher) {
    let config = FormatterConfig::new(120, 4, None);

    bencher.bench(|| {
        format_text(
            divan::black_box(LARGE_DJANGO_TEMPLATE.code),
            divan::black_box(&config),
            divan::black_box(&LARGE_DJANGO_TEMPLATE.profile),
        )
        .expect("Formatting to succeed")
    });
}

#[divan::bench(name = "Nested Django template (0.3k LoC)")]
fn nested_django_template(bencher: divan::Bencher) {
    let config = FormatterConfig::new(120, 4, None);

    bencher.bench(|| {
        format_text(
            divan::black_box(NESTED_DJANGO_TEMPLATE.code),
            divan::black_box(&config),
            divan::black_box(&NESTED_DJANGO_TEMPLATE.profile),
        )
        .expect("Formatting to succeed")
    });
}
