use fc_rpc_core::types::BlockNumber;
use sc_client_api::{BlockBackend, HeaderBackend};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};
use std::{marker::PhantomData, sync::Arc};
/// ethereum request block time to a greater extend, we can ansower some of them locally, lets try!
pub struct BlockMapper<B, C> {
	client: Arc<C>,
	_marker: PhantomData<B>,
}

impl<B, C> BlockMapper<B, C>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
{
	pub fn from_client(client: Arc<C>) -> BlockMapper<B, C> {
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
}
