//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
//! Derived from substrate-node-template

use runtime::{self, opaque::Block, RuntimeApi};
use sc_executor::NativeElseWasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::Telemetry; // TODO: evaluate how we do telemetry

pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
    // TODO: add runtime-benchmark later
    type ExtendHostFunctions = ();

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        runtime::native_version()
    }
}

type FullClient =
    sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// create partial components required to run the chain
pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            sc_finality_grandpa::GrandpaBlockImport<
                FullBackend,
                Block,
                FullClient,
                FullSelectChain,
            >,
            sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
            Option<Telemetry>,
        ),
    >,
    ServiceError,
> {
    unimplemented!()
}

// create service for new-client
pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError> {
    unimplemented!()
}
