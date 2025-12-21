use std::fs;
use std::path::PathBuf;

use djangofmt_lint::{LintDiagnostic, check_ast};
use markup_fmt::Language;
use markup_fmt::parser::Parser;
use miette::{GraphicalReportHandler, GraphicalTheme};
use rayon::prelude::*;

use crate::ExitStatus;
use crate::args::GlobalConfigArgs;
use crate::error::Result;

pub fn check(args: &CheckCommand, _global_options: &GlobalConfigArgs) -> Result<ExitStatus> {
    let results: Vec<(PathBuf, Vec<LintDiagnostic>)> = args
        .files
        .par_iter()
        .filter_map(|path| {
            let source = fs::read_to_string(path).ok()?;
            let mut parser = Parser::new(&source, Language::Jinja, vec![]);
            let ast = parser.parse_root().ok()?;
            let diagnostics = check_ast(&source, &ast);
            if diagnostics.is_empty() {
                None
            } else {
                Some((path.clone(), diagnostics))
            }
        })
        .collect();

    if results.is_empty() {
        return Ok(ExitStatus::Success);
    }

    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());

    for (_path, diagnostics) in &results {
        for diag in diagnostics {
            let mut output = String::new();
            let named_diag = NamedDiagnostic { diag };
            if handler.render_report(&mut output, &named_diag).is_ok() {
                eprint!("{output}");
            }
        }
    }

    Ok(ExitStatus::Failure)
}

struct NamedDiagnostic<'a> {
    diag: &'a LintDiagnostic,
}

impl std::fmt::Debug for NamedDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diag)
    }
}

impl std::fmt::Display for NamedDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diag)
    }
}

impl std::error::Error for NamedDiagnostic<'_> {}

impl miette::Diagnostic for NamedDiagnostic<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(self.diag.code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.diag.help.as_ref().map(|h| Box::new(h.as_str()) as _)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(miette::LabeledSpan::new(
            Some(self.diag.label.clone()),
            self.diag.span.offset(),
            self.diag.span.len(),
        ))))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.diag.source_code as &dyn miette::SourceCode)
    }
}

#[derive(Clone, Debug, clap::Parser)]
pub struct CheckCommand {
    /// List of files to check.
    #[arg(help = "List of files to check", required = true)]
    pub files: Vec<PathBuf>,
}
