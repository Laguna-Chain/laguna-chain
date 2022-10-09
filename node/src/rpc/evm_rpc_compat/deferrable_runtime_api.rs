//! deferrable helper
//!
//! ethereum request will often into either pending tx's or past blocks that might not be there for
//! a non-indexer node, this helper allows the runtime-api to apply tx's in the tx-pool manually and
//! answer the question

use super::pending_api::pending_runtime_api;

use jsonrpsee::core::RpcResult as Result;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;

use std::{marker::PhantomData, sync::Arc};
/// ethereum request block time to a greater extend, we can ansower some of them locally, lets try!
pub struct DeferrableApi<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> DeferrableApi<B, C, A>
where
	A: ChainApi<Block = B> + Sync + Send + 'static,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	pub fn from_client(client: Arc<C>, graph: Arc<Pool<A>>) -> DeferrableApi<B, C, A> {
		DeferrableApi { client, graph, _marker: PhantomData }
	}

	// provide runtime_api that peeks into current tx pool
	pub fn deferrable_runtime_api(&self, pending: bool) -> Result<ApiRef<'_, C::Api>> {
		if !pending {
			Ok(self.client.runtime_api())
		} else {
			pending_runtime_api(self.client.as_ref(), self.graph.as_ref())
		}
	}

	pub fn run_with_api<Out>(
		api: ApiRef<'_, C::Api>,
		execution: impl Fn(ApiRef<'_, C::Api>) -> Out,
	) -> Out {
		execution(api)
	}
}
