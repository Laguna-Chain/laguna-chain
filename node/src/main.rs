//! The first ever laguna-chain cli

pub(crate) use laguna_node::command;

fn main() -> sc_cli::Result<()> {
	command::run()
}
