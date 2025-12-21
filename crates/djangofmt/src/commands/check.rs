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
    let diagnostics: Vec<LintDiagnostic> = args
        .files
        .par_iter()
        .flat_map(|path| {
            let source = fs::read_to_string(path).ok()?;
            let mut parser = Parser::new(&source, Language::Jinja, vec![]);
            let ast = parser.parse_root().ok()?;
            Some(check_ast(&source, &ast))
        })
        .flatten()
        .collect();

    if diagnostics.is_empty() {
        return Ok(ExitStatus::Success);
    }

    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());

    for diag in &diagnostics {
        let mut output = String::new();
        if handler.render_report(&mut output, diag).is_ok() {
            eprint!("{output}");
        }
    }

    Ok(ExitStatus::Failure)
}

#[derive(Clone, Debug, clap::Parser)]
pub struct CheckCommand {
    /// List of files to check.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}
