use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

use djangofmt::args::Args;
use djangofmt::{ExitStatus, run};

// We use jemalloc for performance reasons.
// This has to be kept in sync with the Cargo.toml file section that declares a dependency on tikv-jemallocator.
#[cfg(all(
    not(target_os = "macos"),
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    ),
    feature = "use-jemalloc"
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
#[must_use]
pub fn main() -> ExitCode {
    let args = Args::parse();

    match run(args) {
        Ok(exit_status) => exit_status.into(),
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                // Unhandled error from djangofmt.
                eprintln!("{}", "djangofmt failed".red().bold());
                eprintln!("  {} {err}", "Error:".bold());
            }
            ExitStatus::Error.into()
        }
    }
}
