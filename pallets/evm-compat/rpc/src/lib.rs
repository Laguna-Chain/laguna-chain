//! ## evm-compat-rpc
//!
//! this crate contains various helper methods for clients to query information about
//! pallet-evm-compat

use std::{marker::PhantomData, sync::Arc};

use codec::Codec;

use jsonrpsee::{core::RpcResult, proc_macros::rpc, types::error::CallError};

pub use pallet_evm_compat_rpc_runtime_api::EvmCompatApi as EvmCompatApiRuntimeApi;

use sc_client_api::client::BlockBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	app_crypto::sp_core::H160,
	traits::{Block as BlockT, Header as HeaderT},
};

use fc_rpc_core::types::{Receipt, RichBlock};

#[rpc(server)]
pub trait EvmCompatApi<BlockHash, BlockNumber, AccountId, Balance> {
	#[method(name = "evmCompat_source_to_mapped_address")]
	fn source_to_mapped_address(&self, source: H160, at: Option<BlockHash>)
		-> RpcResult<AccountId>;

	#[method(name = "evmCompat_source_is_backed_by")]
	fn source_is_backed_by(
		&self,
		source: H160,
		at: Option<BlockHash>,
	) -> RpcResult<Option<AccountId>>;

	#[method(name = "evmCompat_check_contract_is_evm_compat")]
	fn check_contract_is_evm_compat(
		&self,
		contract_addr: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<Option<H160>>;
}

/// export ethereum compatible data structures, later consumed by either the relayer, indexer or
/// client from substrate block
#[rpc(server)]
pub trait EvmCompatExportApi<BlockHash> {
	/// export mapped ethereum rich block from substrate block
	#[method(name = "evmCompat_fetch_block")]
	fn fetch_block(&self, at: Option<BlockHash>) -> RpcResult<RichBlock>;

	/// export mapped ethereum transaction receipts from substrate block
	#[method(name = "evmCompat_fetch_receipts")]
	fn fetch_receipts(&self, at: Option<BlockHash>) -> RpcResult<Vec<Receipt>>;
}

pub struct EvmCompatRpc<Client, Block> {
	client: Arc<Client>,
	_marker: PhantomData<Block>,
}

impl<Client, Block> EvmCompatRpc<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<Client, Block, AccountId, Balance>
	EvmCompatApiServer<
		<Block as BlockT>::Hash,
		<<Block as BlockT>::Header as HeaderT>::Number,
		AccountId,
		Balance,
	> for EvmCompatRpc<Client, Block>
where
	Block: BlockT,
	Client: Send
		+ Sync
		+ 'static
		+ ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ BlockBackend<Block>,
	AccountId: Codec,
	Balance: Codec + Copy + Default,
	Client::Api: EvmCompatApiRuntimeApi<Block, AccountId, Balance>,
{
	fn source_to_mapped_address(
		&self,
		source: H160,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<AccountId> {
		let api = self.client.runtime_api();

		api.source_to_mapped_address(at.unwrap_or_else(|| self.client.info().best_hash), source)
			.map_err(|e| CallError::from_std_error(e).into())
	}

	fn source_is_backed_by(
		&self,
		source: H160,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<AccountId>> {
		let api = self.client.runtime_api();
		api.source_is_backed_by(at.unwrap_or_else(|| self.client.info().best_hash), source)
			.map_err(|e| CallError::from_std_error(e).into())
	}

	fn check_contract_is_evm_compat(
		&self,
		contract_addr: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<H160>> {
		let api = self.client.runtime_api();
		api.check_contract_is_evm_compat(
			at.unwrap_or_else(|| self.client.info().best_hash),
			contract_addr,
		)
		.map_err(|e| CallError::from_std_error(e).into())
	}
}
