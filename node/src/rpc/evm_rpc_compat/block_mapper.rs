use super::{block_builder::BlockBuilder, deferrable_runtime_api::DeferrableApi, internal_err};
use ethereum::BlockV2 as EthereumBlock;
use fc_rpc_core::types::BlockNumber;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_service::InPoolTransaction;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
	Digest,
};
use std::{marker::PhantomData, sync::Arc};
/// ethereum request block time to a greater extend, we can ansower some of them locally, lets try!
pub struct BlockMapper<B, C, A: ChainApi> {
	client: Arc<C>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> BlockMapper<B, C, A>
where
	A: ChainApi<Block = B> + 'static + Sync + Send,
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B>,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: BlockBuilderApi<B>,
{
	pub fn from_client(client: Arc<C>) -> BlockMapper<B, C, A> {
		BlockMapper { client, _marker: PhantomData }
	}

	pub(crate) fn map_block(&self, number: Option<BlockNumber>) -> Option<BlockId<B>> {
		let client = &self.client;

		match number.unwrap_or(BlockNumber::Latest) {
			BlockNumber::Hash { hash, .. } => Some(BlockId::<B>::Hash(hash)),
			BlockNumber::Num(number) => Some(BlockId::Number(number.unique_saturated_into())),
			BlockNumber::Latest => Some(BlockId::Hash(client.info().best_hash)),
			BlockNumber::Earliest => Some(BlockId::Hash(client.info().genesis_hash)),
			BlockNumber::Pending => None,
		}
	}

	pub fn reflect_block(&self, number: Option<BlockNumber>) -> Result<Option<EthereumBlock>> {
		let id = self
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		match self.client.block(&id) {
			Ok(Some(b)) => self
				.client
				.runtime_api()
				.map_block(&id, b.block)
				.map(Some)
				.map_err(|e| internal_err(format!("reflect eth block failed: {:?}", e))),
			Ok(None) => Ok(None),
			Err(e) => Err(internal_err(format!("fetch substrate block failed: {:?}", e))),
		}
	}

	pub fn find_author(&self, number: Option<BlockNumber>) -> Result<Option<H160>> {
		self.reflect_block(number).map(|v| v.map(|b| b.header.beneficiary))
	}

	pub fn transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
		let number = BlockNumber::Hash { hash, require_canonical: false };
		self.transaction_count_by_number(number)
	}

	pub fn transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>> {
		self.reflect_block(Some(number)).map(|v| v.map(|b| b.transactions.len().into()))
	}
}
