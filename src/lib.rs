// Library target exists to expose internal modules for integration tests
// and for the xtask binary (man page generation).
// The binary entry point is in main.rs.

mod cli;

pub mod cdp;
pub mod chrome;
pub mod config;
pub mod connection;
pub mod error;
pub mod session;

/// Returns the clap `Command` definition for man page and completion generation.
///
/// This is used by the xtask binary to generate man pages without depending
/// on the binary crate directly.
#[must_use]
pub fn command() -> clap::Command {
    <cli::Cli as clap::CommandFactory>::command()
}
