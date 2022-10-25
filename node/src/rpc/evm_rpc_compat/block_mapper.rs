use fc_rpc_core::types::BlockNumber;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_service::InPoolTransaction;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};

use super::{block_builder::BlockBuilder, internal_err};
use sp_runtime::Digest;
use std::{marker::PhantomData, sync::Arc};
/// ethereum request block time to a greater extend, we can ansower some of them locally, lets try!
pub struct BlockMapper<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> BlockMapper<B, C, A>
where
	A: ChainApi<Block = B>,
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B>,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
{
	pub fn from_client(client: Arc<C>, graph: Arc<Pool<A>>) -> BlockMapper<B, C, A> {
		BlockMapper { client, graph, _marker: PhantomData }
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

	pub fn find_digest(&self, at: &BlockId<B>) -> Result<Vec<([u8; 4], Vec<u8>)>> {
		let header = self.client.header(*at).map_err(|err| {
			internal_err(format!("fetch runtime header digest failed: {:?}", err))
		})?;

		header
			.ok_or_else(|| internal_err("fetch runtime header digest failed"))
			.map(|v| extract_digest(v.digest()))
	}

	pub fn find_author(&self, number: Option<BlockNumber>) -> Result<Option<H160>> {
		let id = self
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let latest_digests = self.find_digest(&id)?;

		self.client
			.runtime_api()
			.author(&id, latest_digests)
			.map_err(|err| internal_err(format!("fetch runtime author failed: {:?}", err)))
	}

	pub fn transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
		let number = BlockNumber::Hash { hash, require_canonical: false };
		self.transaction_count_by_number(number)
	}

	pub fn transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>> {
		if let Some(id) = self.map_block(Some(number)) {
			BlockBuilder::from_client(self.client.clone(), self.graph.clone());

			// Get all transactions from the target block.
			if let Some(txs) = self.client.block_body(&id).map_err(|err| {
				internal_err(format!("fetch runtime block body failed: {:?}", err))
			})? {
				// only count eth-txs
				Ok(Some(
					self.client
						.runtime_api()
						.extrinsic_filter(&id, txs)
						.map_err(|err| {
							internal_err(format!(
								"fetch eth-txs from runtime block body failed: {:?}",
								err
							))
						})?
						.len()
						.into(),
				))
			} else {
				Ok(None)
			}
		} else {
			let pending_txs = self
				.graph
				.validated_pool()
				.ready()
				.map(|t| t.data().clone())
				.collect::<Vec<_>>();

			let latest = BlockId::Hash(self.client.info().best_hash);

			Ok(Some(
				self.client
					.runtime_api()
					.extrinsic_filter(&latest, pending_txs)
					.map_err(|err| {
						internal_err(format!(
							"fetch eth-txs from runtime ready queue failed: {:?}",
							err
						))
					})?
					.len()
					.into(),
			))
		}
	}
}

fn extract_digest(digest: &Digest) -> Vec<([u8; 4], Vec<u8>)> {
	digest
		.logs()
		.iter()
		.filter_map(|v| v.as_consensus().map(|(a, b)| (a, b.to_vec())))
		.collect::<Vec<_>>()
}
