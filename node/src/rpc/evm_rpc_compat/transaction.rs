//! deferrable helper
//!
//! ethereum request will often into either pending tx's or past blocks that might not be there for
//! a non-indexer node, this helper allows the runtime-api to apply tx's in the tx-pool manually and
//! answer the question

use crate::rpc::evm_rpc_compat::internal_err;
use fc_rpc_core::types::{BlockNumber, Bytes, CallRequest};
use fp_rpc::ConvertTransactionRuntimeApi;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi as EvmCompatRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{Backend, HeaderBackend, StateBackend, StorageProvider};
use sc_network::ExHashT;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};

use super::{pending_api::pending_runtime_api, EthApi};

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
}
