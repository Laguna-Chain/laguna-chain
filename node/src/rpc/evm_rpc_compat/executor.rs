//! exectuion helper
//!
//! try call or create

use super::{deferrable_runtime_api::DeferrableApi, BlockMapper};
use crate::rpc::evm_rpc_compat::internal_err;
use fc_rpc_core::types::{BlockNumber, Bytes, CallRequest};
use fp_ethereum::TransactionData;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
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
	B: BlockT<Hash = H256, Extrinsic = UncheckedExtrinsic, Header = Header> + Send + Sync + 'static,
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
		let mapper = BlockMapper::<B, C, A>::from_client(self.client.clone());

		// return none if toward pending
		let id = mapper.map_block(number);

		let deferrable = DeferrableApi::from_client(self.client.clone(), self.graph.clone());
		let deferred_api = deferrable.deferrable_runtime_api(id.is_none())?;

		let id = id.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		DeferrableApi::<B, C, A>::run_with_api(deferred_api, |api| {
			let CallRequest { from, to, value, ref data, gas, gas_price, max_fee_per_gas, .. } =
				request;

			// return (return_value, weight_used) from create, call or transfer
			let (rv, w) = api
				.call(
					&id,
					from,
					to,
					value.map(|v| v.unique_saturated_into()).unwrap_or_default(),
					data.clone().map(|v| v.0.to_vec()).unwrap_or_default(),
					gas.unwrap_or_default(),
					max_fee_per_gas.or(gas_price).unwrap_or_default(),
				)
				.map(|o| {
					o.map_err(|err| internal_err(format!("fetch runtime call failed: {:?}", err)))
				})
				.map_err(|err| internal_err(format!("fetch runtime call failed: {:?}", err)))??;

			Ok((w.into(), Bytes::from(rv.to_vec())))
		})
	}

	pub fn get_code_at(&self, address: H160, number: Option<BlockNumber>) -> Result<Bytes> {
		// self.client.state();
		todo!()
	}
}
