use std::io::{self, BufWriter, Write};

use anyhow::Result;

/// Display version information
pub(crate) fn version() -> Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    // This version is pulled from Cargo.toml and set by Cargo
    let version = env!("CARGO_PKG_VERSION");

    writeln!(stdout, "djangofmt {}", &version)?;
    Ok(())
}
