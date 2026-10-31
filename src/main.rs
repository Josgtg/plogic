mod cli;

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
