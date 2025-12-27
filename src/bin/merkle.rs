//! Merkle CLI Binary
//!
//! Command-line interface for the Merkle filesystem state management system.

use clap::Parser;
use merkle::tooling::cli::{Cli, CliContext};
use std::process;

fn main() {
    let cli = Cli::parse();

    // Create CLI context
    let context = match CliContext::new(cli.workspace.clone(), cli.config.clone()) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Error initializing workspace: {}", e);
            process::exit(1);
        }
    };

    // Execute command
    match context.execute(&cli.command) {
        Ok(output) => {
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
