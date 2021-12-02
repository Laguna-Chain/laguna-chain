// expose rpc, derived from substrate-node-template

use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
use primitives::{AccountId, Balance, Index};

#[cfg(not(feature = "test_runtime"))]
use hydro_runtime::opaque::Block;

#[cfg(feature = "test_runtime")]
use dummy_runtime::opaque::Block;

use std::sync::Arc;
use substrate_frame_rpc_system::{FullSystem, SystemApi};

pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

// TODO: light client before deprecation require additional dependencies

pub struct FullDeps<Client, Pool> {
    pub client: Arc<Client>,
    pub pool: Arc<Pool>,
    pub deny_unsafe: DenyUnsafe,
}

type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// construct and mount all interface to io_handler
/// runtime need meet the requirement by impl the constraint from impl_runtime_apis! macro
pub fn create_full<Client, Pool>(deps: FullDeps<Client, Pool>) -> RpcExtension
// TODO: provide additional rpc interface by adding Client: SomeConstraint
where
    Client: ProvideRuntimeApi<Block>, // should be able to provide runtime-api
    Client: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static, // should be able to handle block header and metadata
    Client: Send + Sync + 'static,
    Client::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>, // client be able to distinquish tx index
    Client::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    Client::Api: BlockBuilder<Block>, // should be able to produce block
    Pool: TransactionPool + 'static,  // can submit tx into tx-pool
{
    let mut io = jsonrpc_core::IoHandler::default();

    let FullDeps {
        client,
        pool,
        deny_unsafe,
    } = deps;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool,
        deny_unsafe,
    )));

    // allow submit transaction by paying the fee
    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    // TODO: extend io with needed rpc here interface

    io
}
