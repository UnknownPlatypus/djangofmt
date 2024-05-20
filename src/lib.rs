use std::fs::File;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use anyhow::Result;
use clap::Parser;
use markup_fmt::{format_text, Language};
use markup_fmt::config::{FormatOptions, LanguageOptions, LayoutOptions};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::args::{Args, Command, FormatCommand};
use crate::logging::set_up_logging;

pub mod args;
mod commands;
mod logging;

#[derive(Copy, Clone)]
pub enum ExitStatus {
    /// Command was successful and there were no errors.
    Success,
    /// Command was successful but there were errors.
    Failure,
    /// Command failed.
    Error,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::Success => ExitCode::from(0),
            ExitStatus::Failure => ExitCode::from(1),
            ExitStatus::Error => ExitCode::from(2),
        }
    }
}

pub fn run(
    Args {
        command,
        global_options,
    }: Args,
) -> Result<ExitStatus> {
    set_up_logging(global_options.log_level())?;

    match command {
        Command::Format(args) => format(args),
        Command::Version => {
            commands::version::version()?;
            Ok(ExitStatus::Success)
        }
    }
}

fn format(args: FormatCommand) -> Result<ExitStatus> {
    let options = FormatOptions {
        layout: LayoutOptions {
            print_width: args.line_length.unwrap_or(120),
            indent_width: 4,
            ..LayoutOptions::default()
        },
        language: LanguageOptions {
            closing_bracket_same_line: false, // This is default, remove later
            ..LanguageOptions::default()
        },
    };

    let start = Instant::now();
    // let (results, mut errors): (Vec<_>, Vec<_>)
    let _results: Vec<_> = args
        .files
        .par_iter()
        .map(|entry| {
            let path = entry.as_path();
            println!("{:?}", path);
            // Extract the source from the file.
            let unformatted_html = std::fs::read_to_string(path).unwrap();

            // Format the source.
            let formatted_html = format_text(
                &unformatted_html,
                Language::Jinja,
                &options,
                |_, code, _| Ok::<_, ()>(code.into()),
            )
            .unwrap();
            let mut writer = File::create(path).unwrap();
            writer.write_all(formatted_html.as_bytes());
            Some(())
        })
        .collect();

    let duration = start.elapsed();
    println!("Formatted {} files in {:.2?}", args.files.len(), duration);
    Ok(ExitStatus::Success)
}
