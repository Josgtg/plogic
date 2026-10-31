mod cli;
// Temporary: the pipeline is not wired to the CLI yet, so the core modules are
// reachable from the tests only. Drop these attributes with the orchestrator.
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod expression;
#[allow(dead_code)]
mod parser;
#[allow(dead_code)]
mod scanner;
#[allow(dead_code)]
mod tokens;

use std::ffi::OsString;

use cli::args;

fn main() {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    match args::parse(argv) {
        // Temporary: the pipeline is not implemented yet, so the parsed
        // command is printed instead of being run.
        Ok(config) => println!("{config:#?}"),
        Err(error) => {
            eprintln!("{}", error.report(cli::error_colors()));
            std::process::exit(2);
        }
    }
}
