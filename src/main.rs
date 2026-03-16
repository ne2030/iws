mod cli;
mod commands;
mod config;
mod workspace;
mod git;
mod spec;
mod claude;
mod tui;

use anyhow::Result;
use cli::Cli;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
