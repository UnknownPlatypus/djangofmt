use djangofmt_benchmark::{
    ALL_TEMPLATES, DJANGO_TEMPLATE_LARGE, FORMATTER_DIRECTIVE, LINT_DIRECTIVE, TestFile, warmup,
    with_directive,
};
use djangofmt_lint::{RuleSet, Settings, parse};

fn main() {
    divan::main();
}

/// `Parsed::check` with no rules: ast traversal only.
#[divan::bench(args = ALL_TEMPLATES)]
fn check_no_rules(bencher: divan::Bencher, template: &'static TestFile) {
    bench_check(
        bencher,
        template,
        &Settings {
            rules: RuleSet::empty(),
            ..Settings::default()
        },
    );
}

/// `Parsed::check` with the default selection (every stable rule, preview off)
#[divan::bench(args = ALL_TEMPLATES)]
fn check_default_rules(bencher: divan::Bencher, template: &'static TestFile) {
    bench_check(bencher, template, &Settings::default());
}

/// `Parsed::check` with all rules, preview included.
#[divan::bench(args = ALL_TEMPLATES)]
fn check_all_rules(bencher: divan::Bencher, template: &'static TestFile) {
    bench_check(bencher, template, &Settings::all());
}

/// The formatter directive must stay free for the linter.
#[divan::bench]
fn check_formatter_directive(bencher: divan::Bencher) {
    bench_directive(bencher, FORMATTER_DIRECTIVE);
}

/// A real suppression: the linter has to locate the guarded node and filter its diagnostics.
#[divan::bench]
fn check_lint_directive(bencher: divan::Bencher) {
    bench_directive(bencher, LINT_DIRECTIVE);
}

fn bench_directive(bencher: divan::Bencher, directive: &str) {
    let settings = Settings::default();
    let source = with_directive(&DJANGO_TEMPLATE_LARGE, directive);
    let parsed =
        parse(&source, DJANGO_TEMPLATE_LARGE.profile.into(), &[]).expect("Parsing to succeed");

    let run =
        || divan::black_box(&parsed).check(divan::black_box(&settings), divan::black_box(None));
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(&source))
        .bench(run);
}

/// Time `Parsed::check` only: the AST is parsed once, outside the timed region.
/// The `check_all_rules` − `check_no_rules` gap is then pure rule-body cost.
fn bench_check(bencher: divan::Bencher, template: &TestFile, settings: &Settings) {
    let parsed = parse(template.code, template.profile.into(), &[]).expect("Parsing to succeed");

    let run =
        || divan::black_box(&parsed).check(divan::black_box(settings), divan::black_box(None));
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(run);
}
