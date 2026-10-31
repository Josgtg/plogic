mod args;
mod cli;
mod error;
mod expression;
mod interpreter;
mod orchestrator;
mod parser;
mod scanner;
mod simplifier;
mod tokens;
mod truth_table;

use std::ffi::OsString;

fn main() {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    match args::parse(argv) {
        Ok(config) => std::process::exit(cli::run(config)),
        Err(error) => {
            eprintln!("{}", error.report(cli::error_colors()));
            std::process::exit(2);
        }
    }
}
