use djangofmt_benchmark::{ALL_TEMPLATES, TestFile};
use markup_fmt::parser::Parser;

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
fn parse_templates(bencher: divan::Bencher, template: &'static TestFile) {
    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(|| {
            let mut parser = Parser::new(
                divan::black_box(template.code),
                divan::black_box(template.profile.into()),
                vec![],
            );
            parser.parse_root().expect("Parsing to succeed")
        });
}
