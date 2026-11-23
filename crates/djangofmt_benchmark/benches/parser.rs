use djangofmt_benchmark::{profile, warmup};
use djangofmt_corpus::{ALL_TEMPLATES, Template};
use djangofmt_syntax::parse;

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
fn parse_templates(bencher: divan::Bencher, template: &'static Template) {
    let language = profile(template).into();
    let run = || {
        parse(
            divan::black_box(template.code),
            divan::black_box(language),
            &[],
        )
        .expect("Parsing to succeed")
    };
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(run);
}
