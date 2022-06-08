//! The first ever laguna-chain cli

pub(crate) use laguna_node::{cli, command, command_helper, rpc, service};

fn main() -> sc_cli::Result<()> {
	command::run()
}
