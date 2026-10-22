mod api;
mod backup;
mod cli;
mod commands;
mod config;
mod filter;
mod mcp;
mod paths;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Init(a)         => commands::init::run(cli.global.clone(), a.clone()),
        Command::UpdateModels(a) => commands::update_models::run(cli.global.clone(), a.clone()),
        Command::Validate        => commands::validate::run(cli.global.clone()),
        Command::Rollback        => commands::rollback::run(cli.global.clone()),
        Command::Format          => commands::format::run(cli.global.clone()),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
