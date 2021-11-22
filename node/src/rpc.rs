// expose rpc, derived from substrate-node-template

use runtime::{opaque::Block, AccountId, Balance, Index};
use std::sync::Arc;

pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

// TODO: light client before deprecation require additional dependencies

pub struct FullDeps<Client, Pool> {
    pub cliet: Arc<Client>,
    pub pool: Arc<Pool>,
    pub deny_unsafe: DenyUnsafe,
}

type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// construct and mount all interface to io_handler
pub fn create_full<Client, Pool>(deps: FullDeps<Client, Pool>) -> RpcExtension
where
    Client: ProvideRuntimeApi<Block>, // should be able to provide runtime-api
    Client: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static, // should be able to handle block header and metadata
    Client: Send + Sync + 'static,
    Client::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>, // client be able to distinquish tx index
    Client::Api: BlockBuilder<Block>, // should be able to produce block
    Pool: TransactionPool + 'static,  // can submit tx into tx-pool
{
    let mut io = jsonrpc_core::IoHandler::default();

    // TODO: extend io with needed rpc here interface

    io
}
