mod app;
mod cli;
mod features;
mod shared;
mod tui;

use anyhow::Result;
use clap::Parser;

use crate::app::App;
use crate::cli::Args;
use crate::shared::bruno::resolve_root;

fn main() -> Result<()> {
    let args = Args::parse();
    let root = resolve_root(args.root)?;
    tui::run(App::new(root)?)
}
