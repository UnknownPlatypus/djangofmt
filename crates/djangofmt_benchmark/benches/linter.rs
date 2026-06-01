use djangofmt_benchmark::{ALL_TEMPLATES, TestFile};
use djangofmt_lint::{PreviewMode, RuleSelector, Settings, check_ast};
use markup_fmt::parser::Parser;

fn main() {
    divan::main();
}

#[divan::bench(args = ALL_TEMPLATES)]
fn check_templates(bencher: divan::Bencher, template: &'static TestFile) {
    // Benchmark every rule, including preview ones, so throughput stays
    // comparable across commits regardless of a rule's stability group.
    let settings =
        Settings::from_selectors(&[RuleSelector::All], &[], &[], &[], PreviewMode::Enabled);

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(|| {
            let mut parser = Parser::new(
                divan::black_box(template.code),
                divan::black_box(template.profile.into()),
                vec![],
            );
            let ast = parser.parse_root().expect("Parsing to succeed");
            check_ast(
                divan::black_box(template.code),
                divan::black_box(&ast),
                divan::black_box(&settings),
            )
        });
}
