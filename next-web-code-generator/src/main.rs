//! Binary entry point for `next-web-code-generator`.

use clap::Parser;
use next_web_code_generator::{Cli, run_cli};

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run_cli(cli) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
