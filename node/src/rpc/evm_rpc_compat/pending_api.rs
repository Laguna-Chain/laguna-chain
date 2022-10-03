use fc_rpc::internal_err;
use jsonrpsee::core::RpcResult as Result;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi as EvmCompatRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{Backend, HeaderBackend, StateBackend, StorageProvider};
use sc_service::InPoolTransaction;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{Core, HeaderT, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::H256;
use sp_runtime::{
	generic::BlockId,
	traits::{BlakeTwo256, Block as BlockT},
};

// NOTICE: derived from frontier
///  this extends the runtime api to consier pending tx's
pub fn pending_runtime_api<'a, B: BlockT, C, BE, A: ChainApi>(
	client: &'a C,
	graph: &'a Pool<A>,
) -> Result<sp_api::ApiRef<'a, C::Api>>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE>,
	C: HeaderBackend<B> + Send + Sync + 'static,
	C::Api: BlockBuilderApi<B> + EvmCompatRuntimeApi<B, AccountId, Balance>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	A: ChainApi<Block = B> + 'static,
{
	// In case of Pending, we need an overlayed state to query over.
	let api = client.runtime_api();
	let best = BlockId::Hash(client.info().best_hash);
	// Get all transactions in the ready queue.
	let xts: Vec<<B as BlockT>::Extrinsic> = graph
		.validated_pool()
		.ready()
		.map(|in_pool_tx| in_pool_tx.data().clone())
		.collect::<Vec<<B as BlockT>::Extrinsic>>();
	// Manually initialize the overlay.
	if let Ok(Some(header)) = client.header(best) {
		let parent_hash = BlockId::Hash(*header.parent_hash());
		api.initialize_block(&parent_hash, &header)
			.map_err(|e| internal_err(format!("Runtime api access error: {:?}", e)))?;
		// Apply the ready queue to the best block's state.
		for xt in xts {
			let _ = api.apply_extrinsic(&best, xt);
		}
		Ok(api)
	} else {
		Err(internal_err(format!("Cannot get header for block {:?}", best)))
	}
}
