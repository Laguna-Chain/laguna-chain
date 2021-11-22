//! cli interface compatible with SubstrateCl

use crate::cli::HydroCli;
use sc_cli::{ChainSpec, RunCmd, RuntimeVersion, SubstrateCli};

// TODO: only scaffolding now
impl SubstrateCli for HydroCli {
    fn impl_name() -> String {
        todo!()
    }

    fn impl_version() -> String {
        todo!()
    }

    fn description() -> String {
        todo!()
    }

    fn author() -> String {
        todo!()
    }

    fn support_url() -> String {
        todo!()
    }

    fn copyright_start_year() -> i32 {
        todo!()
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
        todo!()
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        todo!()
    }
}

pub fn run() -> sc_cli::Result<()> {
    let cli = <HydroCli as SubstrateCli>::from_args();

    // TODO: parse cli and execute corresponding command runner
    match &cli.subcommand {
        Some(_) => todo!(),
        None => todo!(),
    }
}
