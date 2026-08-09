//! Diagnostic rendering: the `miette` path the linter benches stop before, and
//! the dominant cost of `djangofmt check` on a dirty codebase.

use djangofmt_benchmark::{
    DJANGO_TEMPLATE_DEEPLY_NESTED, DJANGO_TEMPLATE_LARGE, JINJA_TEMPLATE_LARGE, TestFile,
};
use djangofmt_lint::{FileDiagnostics, Settings, lint_source};
use miette::{GraphicalReportHandler, GraphicalTheme};

fn main() {
    divan::main();
}

/// Templates that trip enough rules to measure rendering on.
static DIAGNOSTIC_HEAVY: [&TestFile; 3] = [
    &DJANGO_TEMPLATE_DEEPLY_NESTED,
    &DJANGO_TEMPLATE_LARGE,
    &JINJA_TEMPLATE_LARGE,
];

/// Every diagnostic of a file, rendered as its own self-contained block.
#[divan::bench(args = DIAGNOSTIC_HEAVY)]
fn render(bencher: divan::Bencher, template: &'static TestFile) {
    let diagnostics = lint_source(
        template.code,
        template.profile.into(),
        &[],
        &Settings::all(),
    )
    .expect("Parsing to succeed");
    assert!(!diagnostics.is_empty(), "{} tripped no rule", template.name);
    let file_diagnostics = FileDiagnostics::new(template.name, template.code, diagnostics);

    // Pinned to what an interactive `check` renders, so the terminal the
    // benchmark happens to run in can't change the result.
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode()).with_width(120);

    bencher.counter(file_diagnostics.len()).bench(|| {
        let mut out = String::new();
        divan::black_box(&file_diagnostics)
            .render(&handler, &mut out)
            .expect("rendering to a String cannot fail");
        out
    });
}
