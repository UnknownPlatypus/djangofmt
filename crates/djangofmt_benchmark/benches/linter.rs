use djangofmt_benchmark::{
    ALL_TEMPLATES, DJANGO_TEMPLATE_LARGE, FORMATTER_DIRECTIVE, LINT_DIRECTIVE, TestFile, warmup,
    with_directive,
};
use djangofmt_lint::settings::unsorted_tailwind_classes;
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
            unsorted_tailwind_classes: unsorted_tailwind_classes::Settings::default(),
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

/// The formatter directive must stay free for the linter: it carries no lint codes, but it
/// does start with `djangofmt`, and real codebases are full of it.
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
    let ast =
        parse(&source, DJANGO_TEMPLATE_LARGE.profile.into(), &[]).expect("Parsing to succeed");

    let run = || {
        check_ast(
            divan::black_box(source.as_str()),
            divan::black_box(&ast),
            divan::black_box(&settings),
            divan::black_box(None),
        )
    };
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(&source))
        .bench(run);
}

/// Time `check_ast` only: the AST is parsed once, outside the timed region, so
/// parse cost (see `parser::parse_templates`) doesn't swamp the linter signal.
/// The `check_all_rules` − `check_no_rules` gap is then pure rule-body cost.
fn bench_check(bencher: divan::Bencher, template: &TestFile, settings: &Settings) {
    let ast = parse(template.code, template.profile.into(), &[]).expect("Parsing to succeed");

    let run = || {
        check_ast(
            divan::black_box(template.code),
            divan::black_box(&ast),
            divan::black_box(settings),
            divan::black_box(None),
        )
    };
    warmup(run);

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(run);
}
