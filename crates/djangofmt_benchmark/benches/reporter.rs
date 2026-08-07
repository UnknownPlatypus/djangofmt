//! Diagnostic rendering: the `miette` path the linter benches stop before, and
//! the dominant cost of `djangofmt check` on a dirty codebase.

use djangofmt_benchmark::{
    DJANGO_TEMPLATE_DEEPLY_NESTED, DJANGO_TEMPLATE_LARGE, JINJA_TEMPLATE_LARGE, TestFile,
};
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource};

fn main() {
    divan::main();
}

/// Templates that trip enough rules to measure rendering on.
static DIAGNOSTIC_HEAVY: [&TestFile; 3] = [
    &DJANGO_TEMPLATE_DEEPLY_NESTED,
    &DJANGO_TEMPLATE_LARGE,
    &JINJA_TEMPLATE_LARGE,
];

/// Every diagnostic of a file, rendered to the terminal output the user sees.
#[divan::bench(args = DIAGNOSTIC_HEAVY)]
fn render(bencher: divan::Bencher, template: &'static TestFile) {
    let mut parser = Parser::new(template.code, template.profile.into(), vec![]);
    let ast = parser.parse_root().expect("Parsing to succeed");
    let diagnostics = check_ast(template.code, &ast, &Settings::all());
    assert!(!diagnostics.is_empty(), "{} tripped no rule", template.name);
    let file_diagnostics = FileDiagnostics::new(
        NamedSource::new(template.name, template.code.to_string()),
        diagnostics,
    );

    // Pinned to what an interactive `check` renders, so the terminal the
    // benchmark happens to run in can't change the result.
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode()).with_width(120);

    bencher.counter(file_diagnostics.len()).bench(|| {
        let mut out = String::new();
        handler
            .render_report(&mut out, divan::black_box(&file_diagnostics))
            .expect("rendering to a String cannot fail");
        out
    });
}
