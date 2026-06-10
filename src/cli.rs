use std::path::PathBuf;

use clap::Parser;

/// A terminal client for Bruno collections.
#[derive(Debug, Parser)]
#[command(name = "requests-tui", version, about)]
pub struct Args {
    /// Root directory to scan for Bruno collections (folders containing bruno.json).
    /// Given as a positional argument; defaults to the current directory.
    #[arg(value_name = "ROOT", default_value = ".")]
    pub root: PathBuf,
}
