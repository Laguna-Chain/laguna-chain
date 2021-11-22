//! The first ever hydro-chain cli

mod cli;
mod command;

use hydro_node::{chain_spec, service};

fn main() -> sc_cli::Result<()> {
    // TODO: ignite CLI here
    cli::run()
}
