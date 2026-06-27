use clap::Parser;
use rust_course::cli::Cli;
use rust_course::error::AppResult;

fn main() -> AppResult<()> {
    let cli = Cli::parse();
    cli.dispatch()
}
