//! The first ever hydro-chain cli

mod cli;
mod command;

fn main() -> sc_cli::Result<()> {
    command::run()
}
