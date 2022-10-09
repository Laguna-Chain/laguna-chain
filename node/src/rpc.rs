// expose rpc, derived from substrate-node-template

use fp_rpc::ConvertTransactionRuntimeApi;
use laguna_runtime::opaque::Block;
use primitives::{AccountId, Balance, BlockNumber, Hash, Index};
use sp_core::traits::SpawnNamed;
use std::sync::Arc;

use pallet_contracts_rpc::{Contracts, ContractsApiServer, ContractsRuntimeApi};
use pallet_currencies_rpc::{CurrenciesApiServer, CurrenciesRpc, CurrenciesRuntimeApi};
use pallet_evm_compat_rpc::{EvmCompatApiRuntimeApi, EvmCompatApiServer, EvmCompatRpc};

use pallet_transaction_payment_rpc::{
	TransactionPayment, TransactionPaymentApiServer, TransactionPaymentRuntimeApi,
};
use sc_client_api::{
	backend::{Backend, StorageProvider},
	BlockBackend,
};
use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use substrate_frame_rpc_system::{AccountNonceApi, System, SystemApiServer};

mod evm_rpc_compat;
use fc_rpc_core::{EthApiServer, NetApiServer};
use sc_network::NetworkService;
pub struct FullDeps<Client, P, A: ChainApi> {
	pub client: Arc<Client>,
	pub pool: Arc<P>,
	pub graph: Arc<Pool<A>>,
	pub deny_unsafe: DenyUnsafe,
	pub network: Arc<NetworkService<Block, Hash>>,
	pub is_authority: bool,
}
use fc_rpc::EthPubSubApiServer;
use jsonrpsee::RpcModule;
use sc_rpc::SubscriptionTaskExecutor;

type RpcExtension = Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>;

/// construct and mount all interface to io_handler
/// runtime need meet the requirement by impl the constraint from impl_runtime_apis! macro
pub fn create_full<Client, Pool, BE, A>(
	deps: FullDeps<Client, Pool, A>,
	subscription_task_executor: SubscriptionTaskExecutor,
) -> RpcExtension
// TODO: provide additional rpc interface by adding Client: SomeConstraint
where
	BE: Backend<Block> + 'static,
	Client: StorageProvider<Block, BE>,
	Client: BlockBackend<Block>,
	Client: ProvideRuntimeApi<Block>, // should be able to provide runtime-api
	Client: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static, /* should be able to handle block header and metadata */
	Client: Send + Sync + 'static,
	Client::Api: AccountNonceApi<Block, AccountId, Index>, /* client be able to distinquish tx
	                                                        * index */
	Client::Api: TransactionPaymentRuntimeApi<Block, Balance>,
	Client::Api: ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>,
	Client::Api: CurrenciesRuntimeApi<Block, AccountId, Balance>,
	Client::Api: ConvertTransactionRuntimeApi<Block>,
	Client::Api: ConvertTransactionRuntimeApi<Block>,
	Client::Api: EvmCompatApiRuntimeApi<Block, AccountId, Balance>,
	Client::Api: BlockBuilder<Block>, // should be able to produce block
	Pool: TransactionPool<Block = Block> + 'static, // can submit tx into tx-pool
	A: ChainApi<Block = Block> + 'static,
{
	let mut module = RpcModule::new(());

	let FullDeps { client, pool, deny_unsafe, network, is_authority, graph } = deps;

	// ++++++++++++++++
	// operational rpcs
	// ++++++++++++++++

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	// ++++++++++
	// extra rpcs
	// ++++++++++

	module.merge(Contracts::new(client.clone()).into_rpc())?;
	module.merge(CurrenciesRpc::new(client.clone()).into_rpc())?;

	module.merge(EvmCompatRpc::new(client.clone()).into_rpc())?;

	module.merge(
		evm_rpc_compat::net_api::Net::new(client.clone(), network.clone(), true).into_rpc(),
	)?;

	module.merge(
		evm_rpc_compat::pubsub::PubSub::new(
			client.clone(),
			network.clone(),
			subscription_task_executor,
			graph.clone(),
		)
		.into_rpc(),
	)?;

	module.merge(
		evm_rpc_compat::EthApi::new(
			client.clone(),
			pool.clone(),
			graph.clone(),
			network.clone(),
			is_authority,
			Some(laguna_runtime::TransactionConverter),
		)
		.into_rpc(),
	)?;

	Ok(module)
}
