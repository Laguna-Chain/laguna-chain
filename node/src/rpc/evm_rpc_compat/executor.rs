//! exectuion helper
//!
//! try call or create

use super::{deferrable_runtime_api::DeferrableApi, BlockMapper};
use crate::rpc::evm_rpc_compat::internal_err;
use fc_rpc_core::types::{BlockNumber, Bytes, CallRequest};
use jsonrpsee::core::RpcResult as Result;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};
use std::{marker::PhantomData, sync::Arc};

pub struct Execute<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> Execute<B, C, A>
where
	A: ChainApi<Block = B> + Sync + Send + 'static,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	pub fn from_client(client: Arc<C>, graph: Arc<Pool<A>>) -> Execute<B, C, A> {
		Execute { client, graph, _marker: PhantomData }
	}

	pub fn try_call(
		&self,
		request: CallRequest,
		number: Option<BlockNumber>,
	) -> Result<(U256, Bytes)> {
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		// return none is toward pending
		let id = mapper.map_block(number);

		let deferrable = DeferrableApi::from_client(self.client.clone(), self.graph.clone());
		let deferred_api = deferrable.deferrable_runtime_api(id.is_none())?;

		let id = id.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		DeferrableApi::<B, C, A>::run_with_api(deferred_api, |api| {
			let CallRequest { from, to, value, ref data, gas_price, .. } = request;

			// return gas_used and return value from either create or call
			let (f, rv) = api
				.call(
					&id,
					from,
					to,
					value.map(|v| v.unique_saturated_into()).unwrap_or_default(),
					data.clone().map(|v| v.0.to_vec()).unwrap_or_default(),
					gas_price.map(|v| v.unique_saturated_into()).unwrap_or_default(),
				)
				.map(|o| {
					o.map_err(|err| internal_err(format!("fetch runtime call failed: {:?}", err)))
				})
				.map_err(|err| internal_err(format!("fetch runtime call failed: {:?}", err)))??;

			Ok((f.into(), Bytes::from(rv.data.to_vec())))
		})
	}

	pub fn get_code_at(&self, address: H160, number: Option<BlockNumber>) -> Result<Bytes> {
		// self.client.state();
		todo!()
	}
}
