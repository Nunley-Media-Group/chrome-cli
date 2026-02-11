mod cli;
mod error;

use clap::Parser;

use cli::{Cli, Command};
use error::AppError;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        e.print_json_stderr();
        #[allow(clippy::cast_possible_truncation)]
        std::process::exit(e.code as i32);
    }
}

fn run(cli: &Cli) -> Result<(), AppError> {
    match &cli.command {
        Command::Connect => Err(AppError::not_implemented("connect")),
        Command::Tabs => Err(AppError::not_implemented("tabs")),
        Command::Navigate => Err(AppError::not_implemented("navigate")),
        Command::Page => Err(AppError::not_implemented("page")),
        Command::Dom => Err(AppError::not_implemented("dom")),
        Command::Js => Err(AppError::not_implemented("js")),
        Command::Console => Err(AppError::not_implemented("console")),
        Command::Network => Err(AppError::not_implemented("network")),
        Command::Interact => Err(AppError::not_implemented("interact")),
        Command::Form => Err(AppError::not_implemented("form")),
        Command::Emulate => Err(AppError::not_implemented("emulate")),
        Command::Perf => Err(AppError::not_implemented("perf")),
    }
}
