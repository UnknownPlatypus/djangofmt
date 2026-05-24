//! Internal CLI for djangofmt developers.
//!
//! Run with `cargo run -p djangofmt_dev -- <subcommand>`.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::Path;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod generate_all;
mod generate_docs;
mod generate_rules_table;
mod sync_top_level;

/// Workspace root, derived from this crate's manifest dir without `..` segments
/// so paths joined onto it remain clean for [`djangofmt::fs::relativize_path`].
fn root_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("djangofmt_dev manifest dir has two ancestors")
}

const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");
const REPO_BRANCH: &str = "main";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
enum Command {
    /// Run all code and documentation generation steps.
    GenerateAll(generate_all::Args),
    /// Generate per-rule Markdown documentation under `docs/rules/`.
    GenerateDocs(generate_all::Args),
    /// Generate the rules index page at `docs/rules.md`.
    GenerateRulesTable(generate_all::Args),
    /// Sync README, CHANGELOG and CONTRIBUTING into `docs/`.
    SyncTopLevel(generate_all::Args),
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::GenerateAll(args) => generate_all::main(&args),
        Command::GenerateDocs(args) => generate_docs::main(&args),
        Command::GenerateRulesTable(args) => generate_rules_table::main(&args),
        Command::SyncTopLevel(args) => sync_top_level::main(&args),
    }
}
