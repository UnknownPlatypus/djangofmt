use djangofmt_benchmark::{ALL_TEMPLATES, TestFile};
use djangofmt_lint::parse;

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
fn parse_templates(bencher: divan::Bencher, template: &'static TestFile) {
    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(|| {
            parse(
                divan::black_box(template.code),
                divan::black_box(template.profile.into()),
                &[],
            )
            .expect("Parsing to succeed")
        });
}
