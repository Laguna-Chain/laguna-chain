//! cli interface compatible with SubstrateCl
use super::command::Subcommand;
use sc_cli::{ChainSpec, RunCmd, RuntimeVersion, SubstrateCli};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct HydroCli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

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
    // TODO: parse cli and execute corresponding command runner
    unimplemented!()
}
