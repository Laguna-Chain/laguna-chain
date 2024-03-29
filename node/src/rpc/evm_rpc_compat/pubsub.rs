//! pubsub helper

use super::block_builder;
use ethereum::{BlockV2 as EthereumBlock, ReceiptV3};
use fc_rpc::EthPubSubApiServer;
use futures::StreamExt;

use jsonrpsee::SubscriptionSink;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, BlockchainEvents, HeaderBackend};
use sc_network::{ExHashT, NetworkService};
use sc_rpc::SubscriptionTaskExecutor;
use sc_service::{InPoolTransaction, TransactionPool};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{keccak_256, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use fc_rpc_core::types::{
	pubsub::{Kind, Params, PubSubSyncStatus, Result as PubSubResult, SyncStatusMetadata},
	BlockNumber, Bytes, FilteredParams, Log, Rich, RichBlock,
};

use super::block_mapper::BlockMapper;

pub struct PubSub<B: BlockT, C, A: ChainApi, H: ExHashT, P> {
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	pool: Arc<P>,
	graph: Arc<Pool<A>>,
	starting_block: u64,
	subscriptions: SubscriptionTaskExecutor,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A, H: ExHashT, P> PubSub<B, C, A, H, P>
where
	A: ChainApi<Block = B> + Sync + Send + 'static,
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + BlockchainEvents<B>,
	C::Api: BlockBuilderApi<B>,
	P: TransactionPool<Block = B> + Send + Sync + 'static,
{
	pub fn new(
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		pool: Arc<P>,
		subscriptions: SubscriptionTaskExecutor,
		graph: Arc<Pool<A>>,
	) -> PubSub<B, C, A, H, P> {
		let starting_block =
			UniqueSaturatedInto::<u64>::unique_saturated_into(client.info().best_number);

		PubSub { client, graph, network, starting_block, pool, _marker: PhantomData, subscriptions }
	}

	async fn new_heads(mut sink: SubscriptionSink, client: Arc<C>) {
		let stream = client
			.import_notification_stream()
			.filter_map(move |notification| {
				if notification.is_new_best {
					let builder =
						block_builder::BlockBuilder::<B, C, A>::from_client(client.clone());

					let b = builder
						.to_rich_block(
							Some(BlockNumber::Hash {
								require_canonical: false,
								hash: notification.hash,
							}),
							true,
						)
						.ok();

					futures::future::ready(b)
				} else {
					futures::future::ready(None)
				}
			})
			.map(|block| SubscriptionResult::new().new_heads(block));
		sink.pipe_from_stream(stream).await;
	}

	async fn logs(mut sink: SubscriptionSink, filtered_params: FilteredParams, client: Arc<C>) {
		let stream = client
			.import_notification_stream()
			.filter_map(move |notification| {
				if notification.is_new_best {
					let builder =
						block_builder::BlockBuilder::<B, C, A>::from_client(client.clone());

					let bn = Some(BlockNumber::Hash {
						require_canonical: false,
						hash: notification.hash,
					});
					let mapper = BlockMapper::<B, C, A>::from_client(client.clone());

					if let (Ok(Some(b)), Ok(rs)) = (
						mapper.reflect_block(bn),
						builder.receipts(bn).map(|rs| rs.into_iter().collect::<Vec<_>>()),
					) {
						return futures::future::ready(Some((b, rs, notification.hash)))
					}
				}

				futures::future::ready(None)
			})
			.flat_map(move |(block, receipts, block_hash)| {
				futures::stream::iter(SubscriptionResult::new().logs(
					block,
					block_hash,
					receipts,
					&filtered_params,
				))
			})
			.map(|x| PubSubResult::Log(Box::new(x)));
		sink.pipe_from_stream(stream).await;
	}

	async fn new_pending(mut sink: SubscriptionSink, client: Arc<C>, pool: Arc<P>) {
		let stream = pool
			.import_notification_stream()
			.filter_map(move |txhash| {
				if let Some(xt) = pool.ready_transaction(&txhash) {
					let best_block: BlockId<B> = BlockId::Hash(client.info().best_hash);

					let api = client.runtime_api();

					let xts = vec![xt.data().clone()];

					// we only care about eth pending's
					let pendings = api.extrinsic_filter(&best_block, xts).ok();

					let res = match pendings {
						Some(txs) =>
							if txs.len() == 1 {
								Some(txs[0].clone())
							} else {
								None
							},
						_ => None,
					};
					futures::future::ready(res)
				} else {
					futures::future::ready(None)
				}
			})
			.map(|transaction| PubSubResult::TransactionHash(transaction.hash()));
		sink.pipe_from_stream(stream).await;
	}

	async fn syncing(
		mut sink: SubscriptionSink,
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		starting_block: u64,
	) {
		let client = Arc::clone(&client);
		let network = Arc::clone(&network);
		// Gets the node syncing status.
		// The response is expected to be serialized either as a plain boolean
		// if the node is not syncing, or a structure containing syncing metadata
		// in case it is.
		async fn status<C: HeaderBackend<B>, B: BlockT, H: ExHashT + Send + Sync>(
			client: Arc<C>,
			network: Arc<NetworkService<B, H>>,
			starting_block: u64,
		) -> PubSubSyncStatus {
			if network.is_major_syncing() {
				// Get the target block to sync.
				// This value is only exposed through substrate async Api
				// in the `NetworkService`.
				let highest_block = network
					.status()
					.await
					.ok()
					.and_then(|res| res.best_seen_block)
					.map(UniqueSaturatedInto::<u64>::unique_saturated_into);

				// Best imported block.
				let current_block =
					UniqueSaturatedInto::<u64>::unique_saturated_into(client.info().best_number);

				PubSubSyncStatus::Detailed(SyncStatusMetadata {
					syncing: true,
					starting_block,
					current_block,
					highest_block,
				})
			} else {
				PubSubSyncStatus::Simple(false)
			}
		}
		// On connection subscriber expects a value.
		// Because import notifications are only emitted when the node is synced or
		// in case of reorg, the first event is emited right away.
		let _ = sink.send(&PubSubResult::SyncState(
			status(Arc::clone(&client), Arc::clone(&network), starting_block).await,
		));

		// When the node is not under a major syncing (i.e. from genesis), react
		// normally to import notifications.
		//
		// Only send new notifications down the pipe when the syncing status changed.
		let mut stream = client.clone().import_notification_stream();
		let mut last_syncing_status = network.is_major_syncing();
		while (stream.next().await).is_some() {
			let syncing_status = network.is_major_syncing();
			if syncing_status != last_syncing_status {
				let _ = sink.send(&PubSubResult::SyncState(
					status(client.clone(), network.clone(), starting_block).await,
				));
			}
			last_syncing_status = syncing_status;
		}
	}
}

struct SubscriptionResult {}

impl SubscriptionResult {
	pub fn new() -> Self {
		SubscriptionResult {}
	}
	pub fn new_heads(&self, block: RichBlock) -> PubSubResult {
		PubSubResult::Header(Box::new(Rich {
			inner: block.header.clone(),
			extra_info: BTreeMap::new(),
		}))
	}

	pub fn logs(
		&self,
		block: EthereumBlock,
		block_hash: H256,
		receipts: Vec<ethereum::ReceiptV3>,
		params: &FilteredParams,
	) -> Vec<Log> {
		let block_number = block.header.number;

		let mut logs: Vec<Log> = vec![];
		let mut log_index: u32 = 0;

		for (receipt_index, receipt) in receipts.into_iter().enumerate() {
			let receipt_logs = match receipt {
				ethereum::ReceiptV3::Legacy(d) |
				ethereum::ReceiptV3::EIP2930(d) |
				ethereum::ReceiptV3::EIP1559(d) => d.logs,
			};
			let transaction_hash: Option<H256> = if receipt_logs.is_empty() {
				Some(block.transactions[receipt_index as usize].hash())
			} else {
				None
			};

			for (tx_log_idx, log) in receipt_logs.into_iter().enumerate() {
				if self.add_log(block_hash, &log, &block, params) {
					logs.push(Log {
						address: log.address,
						topics: log.topics,
						data: Bytes(log.data),
						block_hash: Some(block_hash),
						block_number: Some(block.header.number),
						transaction_hash,
						transaction_index: Some(U256::from(receipt_index)),
						log_index: Some(U256::from(log_index)),
						transaction_log_index: Some(U256::from(tx_log_idx)),
						removed: false,
					});
				}
				log_index += 1;
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

impl<B, C, A, H: ExHashT, P> EthPubSubApiServer for PubSub<B, C, A, H, P>
where
	A: ChainApi<Block = B> + Sync + Send + 'static,
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C: BlockchainEvents<B>,
	C::Api: BlockBuilderApi<B>,
	P: TransactionPool<Block = B> + Send + Sync + 'static,
{
	fn subscribe(
		&self,
		sink: jsonrpsee::PendingSubscription,
		kind: fc_rpc_core::types::pubsub::Kind,
		params: Option<fc_rpc_core::types::pubsub::Params>,
	) {
		let sink = if let Some(sink) = sink.accept() { sink } else { return };

		let filtered_params = match params {
			Some(Params::Logs(filter)) => FilteredParams::new(Some(filter)),
			_ => FilteredParams::default(),
		};

		let client = self.client.clone();
		let network = self.network.clone();
		let pool = self.pool.clone();
		let starting_block = self.starting_block;

		let fut = async move {
			match kind {
				Kind::NewHeads => {
					Self::new_heads(sink, client).await;
				},
				Kind::Logs => {
					Self::logs(sink, filtered_params, client).await;
				},
				Kind::NewPendingTransactions => {
					Self::new_pending(sink, client, pool).await;
				},
				Kind::Syncing => {
					Self::syncing(sink, client, network, starting_block).await;
				},
			}
		};

		self.subscriptions
			.spawn("frontier-rpc-subscription", Some("rpc"), Box::pin(fut));
	}
}
