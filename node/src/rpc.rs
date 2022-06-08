// expose rpc, derived from substrate-node-template

use laguna_runtime::opaque::Block;
use primitives::{AccountId, Balance, BlockNumber, Hash, Index};
use std::sync::Arc;

use pallet_contracts_rpc::{ContractsApiServer, ContractsRpc, ContractsRuntimeApi};
use pallet_currencies_rpc::{CurrenciesApiServer, CurrenciesRpc, CurrenciesRuntimeApi};

use pallet_transaction_payment_rpc::{
	TransactionPaymentApiServer, TransactionPaymentRpc, TransactionPaymentRuntimeApi,
};
use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use substrate_frame_rpc_system::{AccountNonceApi, SystemApiServer, SystemRpc};

// TODO: light client before deprecation require additional dependencies

pub struct FullDeps<Client, Pool> {
	pub client: Arc<Client>,
	pub pool: Arc<Pool>,
	pub deny_unsafe: DenyUnsafe,
}
use jsonrpsee::RpcModule;

type RpcExtension = Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>;

/// construct and mount all interface to io_handler
/// runtime need meet the requirement by impl the constraint from impl_runtime_apis! macro
pub fn create_full<Client, Pool>(deps: FullDeps<Client, Pool>) -> RpcExtension
// TODO: provide additional rpc interface by adding Client: SomeConstraint
where
	Client: ProvideRuntimeApi<Block>, // should be able to provide runtime-api
	Client: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static, /* should be able to handle block header and metadata */
	Client: Send + Sync + 'static,
	Client::Api: AccountNonceApi<Block, AccountId, Index>, /* client be able to distinquish tx
	                                                        * index */
	Client::Api: TransactionPaymentRuntimeApi<Block, Balance>,
	Client::Api: ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>,
	Client::Api: CurrenciesRuntimeApi<Block, AccountId, Balance>,
	Client::Api: BlockBuilder<Block>, // should be able to produce block
	Pool: TransactionPool + 'static,  // can submit tx into tx-pool
{
	let mut module = RpcModule::new(());

	let FullDeps { client, pool, deny_unsafe } = deps;

	module.merge(SystemRpc::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPaymentRpc::new(client.clone()).into_rpc())?;
	module.merge(ContractsRpc::new(client.clone()).into_rpc())?;

	// // TODO: extend io with needed rpc here interface
	module.merge(CurrenciesRpc::new(client.clone()).into_rpc())?;

	Ok(module)
}
