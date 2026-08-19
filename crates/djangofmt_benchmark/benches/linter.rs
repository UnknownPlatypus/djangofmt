use djangofmt_benchmark::{ALL_TEMPLATES, TestFile};
use djangofmt_lint::{RuleSet, Settings, check_ast, parse};

fn main() {
    divan::main();
}

/// `check_ast` with no rules: the traversal floor.
#[divan::bench(args = ALL_TEMPLATES)]
fn check_no_rules(bencher: divan::Bencher, template: &'static TestFile) {
    bench_check(
        bencher,
        template,
        &Settings {
            rules: RuleSet::empty(),
        },
    );
}

/// `check_ast` with the default selection (every stable rule, preview off): what a user
/// with no rule config pays, so opt-in preview rules don't move this number.
#[divan::bench(args = ALL_TEMPLATES)]
fn check_default_rules(bencher: divan::Bencher, template: &'static TestFile) {
    bench_check(bencher, template, &Settings::default());
}

/// `check_ast` with all rules, preview included: traversal + every rule body.
#[divan::bench(args = ALL_TEMPLATES)]
fn check_all_rules(bencher: divan::Bencher, template: &'static TestFile) {
    bench_check(bencher, template, &Settings::all());
}

/// Time `check_ast` only: the AST is parsed once, outside the timed region, so
/// parse cost (see `parser::parse_templates`) doesn't swamp the linter signal.
/// The `check_all_rules` − `check_no_rules` gap is then pure rule-body cost.
fn bench_check(bencher: divan::Bencher, template: &TestFile, settings: &Settings) {
    let ast = parse(template.code, template.profile.into(), &[]).expect("Parsing to succeed");

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(|| {
            check_ast(
                divan::black_box(template.code),
                divan::black_box(&ast),
                divan::black_box(settings),
                divan::black_box(None),
            )
        });
}
