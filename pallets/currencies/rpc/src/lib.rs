use std::{marker::PhantomData, ops::Deref, sync::Arc};

use codec::Codec;

use jsonrpc_core::{Error as RpcError, ErrorCode, Result as RpcResult};
use jsonrpc_derive::rpc;

use pallet_currencies_runtime_api::CurrenciesApi as CurrenciesRuntimeApi;
use sp_api::{BlockId, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_std::vec::Vec;

use primitives::CurrencyId;

#[rpc(client, server)]
pub trait CurrenciesApi<BlockHash, BlockNumber, AccountId, Balance> {
	#[rpc(name = "currencies_listAssets")]
	fn list_assets(&self, at: Option<BlockHash>) -> RpcResult<Vec<CurrencyId>>;

	#[rpc(name = "currencies_freeBalance")]
	fn free_balance(
		&self,
		account: AccountId,
		currency_id: CurrencyId,
		at: Option<BlockHash>,
	) -> RpcResult<Balance>;

	#[rpc(name = "currencies_totalBalance")]
	fn total_balance(
		&self,
		account: AccountId,
		currency_id: CurrencyId,
		at: Option<BlockHash>,
	) -> RpcResult<Balance>;
}

pub struct CurrenciesRpc<Client, Block> {
	client: Arc<Client>,
	_marker: PhantomData<Block>,
}

impl<Client, Block> CurrenciesRpc<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

const RUNTIME_ERROR: i64 = 1000;

impl<Client, Block, AccountId, Balance>
	CurrenciesApi<
		<Block as BlockT>::Hash,
		<<Block as BlockT>::Header as HeaderT>::Number,
		AccountId,
		Balance,
	> for CurrenciesRpc<Client, Block>
where
	Block: BlockT,
	Client: Send + Sync + 'static,
	Client: ProvideRuntimeApi<Block>,
	Client: HeaderBackend<Block>,
	AccountId: Codec,
	Balance: Codec + Copy + Default,
	Client::Api: CurrenciesRuntimeApi<Block, AccountId, Balance>,
{
	fn list_assets(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<CurrencyId>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.list_assets(&at).map_err(|e| RpcError {
			code: ErrorCode::ServerError(RUNTIME_ERROR),
			message: "Runtime trapped".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn free_balance(
		&self,
		account: AccountId,
		currency_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.free_balance(&at, account, currency_id)
			.map(|v| v.unwrap_or_default())
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(RUNTIME_ERROR),
				message: "Runtime trapped".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn total_balance(
		&self,
		account: AccountId,
		currency_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.total_balance(&at, account, currency_id)
			.map(|v| v.unwrap_or_default())
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(RUNTIME_ERROR),
				message: "Runtime trapped".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}
}
