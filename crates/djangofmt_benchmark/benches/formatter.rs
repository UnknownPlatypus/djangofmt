use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt_benchmark::{LARGE_DJANGO_TEMPLATE, LARGE_JINJA_TEMPLATE, NESTED_DJANGO_TEMPLATE};

fn main() {
    divan::main();
}

#[divan::bench(name = "Large jinja template (2.7k LoC)", args = [1, 120, 100_000_000])]
fn large_jinja_template(bencher: divan::Bencher, print_width: usize) {
    let config = FormatterConfig::new(print_width, 4, None);

    bencher.bench_local(|| {
        format_text(
            divan::black_box(LARGE_JINJA_TEMPLATE.code),
            divan::black_box(&config),
            divan::black_box(&LARGE_JINJA_TEMPLATE.profile),
        )
        .expect("Formatting to succeed")
    });
}

#[divan::bench(name = "Large Django template (1.3k LoC)", args = [1, 120, 100_000_000])]
fn large_django_template(bencher: divan::Bencher, print_width: usize) {
    let config = FormatterConfig::new(print_width, 4, None);

    bencher.bench_local(|| {
        format_text(
            divan::black_box(LARGE_DJANGO_TEMPLATE.code),
            divan::black_box(&config),
            divan::black_box(&LARGE_DJANGO_TEMPLATE.profile),
        )
        .expect("Formatting to succeed")
    });
}

#[divan::bench(name = "Nested Django template (0.3k LoC)", args = [1, 120, 100_000_000])]
fn nested_django_template(bencher: divan::Bencher, print_width: usize) {
    let config = FormatterConfig::new(print_width, 4, None);

    bencher.bench_local(|| {
        format_text(
            divan::black_box(NESTED_DJANGO_TEMPLATE.code),
            divan::black_box(&config),
            divan::black_box(&NESTED_DJANGO_TEMPLATE.profile),
        )
        .expect("Formatting to succeed")
    });
}
