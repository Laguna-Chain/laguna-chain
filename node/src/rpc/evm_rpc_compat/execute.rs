//! deferrable helper
//!
//! ethereum request will often into either pending tx's or past blocks that might not be there for
//! a non-indexer node, this helper allows the runtime-api to apply tx's in the tx-pool manually and
//! answer the question

use crate::rpc::evm_rpc_compat::internal_err;
use fc_rpc_core::types::{BlockNumber, Bytes, CallRequest, FeeHistory};
use fp_rpc::ConvertTransactionRuntimeApi;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use pallet_contracts_primitives::ContractExecResult;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi as EvmCompatRuntimeApi;
use pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{Backend, HeaderBackend, StateBackend, StorageProvider};
use sc_network::ExHashT;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{BlakeTwo256, Block as BlockT, UniqueSaturatedInto},
};

use super::{pending_api::pending_runtime_api, BlockMapper, EthApi};

impl<B, C, H: ExHashT, CT, BE, P, A> EthApi<B, C, H, CT, BE, P, A>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C::Api: ConvertTransactionRuntimeApi<B>,
	C::Api: EvmCompatRuntimeApi<B, AccountId, Balance>,
	C::Api: BlockBuilderApi<B>,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	CT: fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
	P: TransactionPool<Block = B> + Send + Sync + 'static,
	A: ChainApi<Block = B> + 'static,
{
	pub fn try_call(
		&self,
		request: CallRequest,
		number: Option<BlockNumber>,
	) -> Result<(U256, Bytes)> {
		let mapper = BlockMapper::from_client(self.client.clone());

		// return none is toward pending
		let id = mapper.map_block(number);
		let deferred_api = self.deferrable_runtime_api(id.is_none())?;

		let id = id.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		Self::run_with_api(deferred_api, |api| {
			let CallRequest { from, to, value, ref data, gas_price, .. } = request;

			// return gas_used and return value from either create or call
			let (g, rv) = api
				.call(
					&id,
					from,
					to,
					value.map(|v| v.unique_saturated_into()).unwrap_or_default(),
					data.clone().map(|v| v.0.to_vec()).unwrap_or_default(),
					gas_price.map(|v| v.unique_saturated_into()).unwrap_or_default(),
					None,
				)
				.map(|o| {
					o.map_err(|err| internal_err(format!("fetch runtime call failed: {:?}", err)))
				})
				.map_err(|err| internal_err(format!("fetch runtime call failed: {:?}", err)))??;

			Ok((g.into(), Bytes::from(rv.data.to_vec())))
		})
	}
}