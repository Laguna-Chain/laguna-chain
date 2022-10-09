//! pubsub helper

use super::{deferrable_runtime_api::DeferrableApi, BlockMapper};
use crate::rpc::evm_rpc_compat::internal_err;
use ethereum::{BlockV2 as EthereumBlock, TransactionV2 as EthereumTransaction};
use fc_rpc::EthPubSubApiServer;
use fc_rpc_core::types::{BlockNumber, Bytes, CallRequest, Rich};
use futures::FutureExt;
use jsonrpsee::core::RpcResult as Result;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_network::{ExHashT, NetworkService};
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{keccak_256, H160, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use fc_rpc_core::types::{
	pubsub::{Kind, Params, PubSubSyncStatus, Result as PubSubResult, SyncStatusMetadata},
	FilteredParams, Header, Log,
};

pub struct PubSub<B: BlockT, C, A: ChainApi, H: ExHashT> {
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	graph: Arc<Pool<A>>,
	starting_block: u64,
	subscriptions: SubscriptionTaskExecutor,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A, H: ExHashT> PubSub<B, C, A, H>
where
	A: ChainApi<Block = B> + Sync + Send + 'static,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	pub fn new(
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		subscriptions: SubscriptionTaskExecutor,
		graph: Arc<Pool<A>>,
	) -> PubSub<B, C, A, H> {
		let starting_block =
			UniqueSaturatedInto::<u64>::unique_saturated_into(client.info().best_number);

		PubSub { client, graph, network, starting_block, _marker: PhantomData, subscriptions }
	}
}

struct SubscriptionResult {}
impl SubscriptionResult {
	pub fn new() -> Self {
		SubscriptionResult {}
	}
	pub fn new_heads(&self, block: EthereumBlock) -> PubSubResult {
		PubSubResult::Header(Box::new(Rich {
			inner: Header {
				hash: Some(H256::from(keccak_256(&rlp::encode(&block.header)))),
				parent_hash: block.header.parent_hash,
				uncles_hash: block.header.ommers_hash,
				author: block.header.beneficiary,
				miner: block.header.beneficiary,
				state_root: block.header.state_root,
				transactions_root: block.header.transactions_root,
				receipts_root: block.header.receipts_root,
				number: Some(block.header.number),
				gas_used: block.header.gas_used,
				gas_limit: block.header.gas_limit,
				extra_data: Bytes(block.header.extra_data.clone()),
				logs_bloom: block.header.logs_bloom,
				timestamp: U256::from(block.header.timestamp),
				difficulty: block.header.difficulty,
				nonce: Some(block.header.nonce),
				size: Some(U256::from(rlp::encode(&block.header).len() as u32)),
			},
			extra_info: BTreeMap::new(),
		}))
	}
	pub fn logs(
		&self,
		block: EthereumBlock,
		receipts: Vec<ethereum::ReceiptV3>,
		params: &FilteredParams,
	) -> Vec<Log> {
		let block_hash = Some(H256::from(keccak_256(&rlp::encode(&block.header))));
		let mut logs: Vec<Log> = vec![];
		let mut log_index: u32 = 0;
		for (receipt_index, receipt) in receipts.into_iter().enumerate() {
			let receipt_logs = match receipt {
				ethereum::ReceiptV3::Legacy(d) |
				ethereum::ReceiptV3::EIP2930(d) |
				ethereum::ReceiptV3::EIP1559(d) => d.logs,
			};
			let mut transaction_log_index: u32 = 0;
			let transaction_hash: Option<H256> = if receipt_logs.len() > 0 {
				Some(block.transactions[receipt_index as usize].hash())
			} else {
				None
			};
			for log in receipt_logs {
				if self.add_log(block_hash.unwrap(), &log, &block, params) {
					logs.push(Log {
						address: log.address,
						topics: log.topics,
						data: Bytes(log.data),
						block_hash,
						block_number: Some(block.header.number),
						transaction_hash,
						transaction_index: Some(U256::from(receipt_index)),
						log_index: Some(U256::from(log_index)),
						transaction_log_index: Some(U256::from(transaction_log_index)),
						removed: false,
					});
				}
				log_index += 1;
				transaction_log_index += 1;
			}
		}
		logs
	}
	fn add_log(
		&self,
		block_hash: H256,
		ethereum_log: &ethereum::Log,
		block: &EthereumBlock,
		params: &FilteredParams,
	) -> bool {
		let log = Log {
			address: ethereum_log.address,
			topics: ethereum_log.topics.clone(),
			data: Bytes(ethereum_log.data.clone()),
			block_hash: None,
			block_number: None,
			transaction_hash: None,
			transaction_index: None,
			log_index: None,
			transaction_log_index: None,
			removed: false,
		};
		if params.filter.is_some() {
			let block_number =
				UniqueSaturatedInto::<u64>::unique_saturated_into(block.header.number);
			if !params.filter_block_range(block_number) ||
				!params.filter_block_hash(block_hash) ||
				!params.filter_address(&log) ||
				!params.filter_topics(&log)
			{
				return false
			}
		}
		true
	}
}

impl<B, C, A, H: ExHashT> EthPubSubApiServer for PubSub<B, C, A, H>
where
	A: ChainApi<Block = B> + Sync + Send + 'static,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	fn subscribe(
		&self,
		sink: jsonrpsee::PendingSubscription,
		kind: fc_rpc_core::types::pubsub::Kind,
		params: Option<fc_rpc_core::types::pubsub::Params>,
	) {
		let mut sink = if let Some(sink) = sink.accept() { sink } else { return };

		let filtered_params = match params {
			Some(Params::Logs(filter)) => FilteredParams::new(Some(filter)),
			_ => FilteredParams::default(),
		};

		let client = self.client.clone();
		let network = self.network.clone();
		let starting_block = self.starting_block;

		let fut = async move {};

		self.subscriptions
			.spawn("frontier-rpc-subscription", Some("rpc"), fut.map(drop).boxed());
	}
}
