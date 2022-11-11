//! transaction helper
//!
//! helper functions for transaction details

use super::{block_builder, internal_err};
use ethereum::ReceiptV3 as EthereumReceipt;
use fc_rpc_core::types::{BlockNumber, BlockTransactions, Index, Log, Receipt, Transaction};
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_service::InPoolTransaction;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::H256;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::{marker::PhantomData, sync::Arc};

pub struct TransactionApi<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> TransactionApi<B, C, A>
where
	A: ChainApi<Block = B> + 'static + Sync + Send,
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic>,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B> + 'static + Sync + Send,
{
	pub fn from_client(client: Arc<C>, graph: Arc<Pool<A>>) -> TransactionApi<B, C, A> {
		TransactionApi { client, graph, _marker: PhantomData }
	}

	pub async fn get_transaction_by_block_number_and_index(
		&self,
		number: BlockNumber,
		index: Index,
	) -> Result<Option<Transaction>> {
		let builder = block_builder::BlockBuilder::<B, C, A>::from_client(self.client.clone());

		let rich_block = builder.to_rich_block(Some(number), true)?;

		if let BlockTransactions::Full(txs) = &rich_block.transactions {
			Ok(txs.iter().find(|v| v.transaction_index == Some(index.value().into())).cloned())
		} else {
			Ok(None)
		}
	}

	pub async fn get_transaction_by_block_hash_and_index(
		&self,
		hash: H256,
		index: Index,
	) -> Result<Option<Transaction>> {
		let bn = BlockNumber::Hash { hash, require_canonical: false };
		self.get_transaction_by_block_number_and_index(bn, index).await
	}

	pub fn get_transaction_from_blocks(&self, hash: H256) -> Result<Option<Transaction>> {
		let mut latest = BlockId::<B>::hash(self.client.info().best_hash);
		let builder = block_builder::BlockBuilder::<B, C, A>::from_client(self.client.clone());

		// NOTICE: this is needed to avoid hanging query forever
		// allow query up to 1024 past blocks
		for _ in 0..1024 {
			if let Ok(Some(header)) = self.client.header(latest) {
				let bn = Some(BlockNumber::Hash { hash: header.hash(), require_canonical: false });
				// prepare the mapped rich_block
				let rich_block = builder.to_rich_block(bn, true)?;

				let txs = if let BlockTransactions::Full(txs) = &rich_block.transactions {
					Some(txs.clone())
				} else {
					None
				};

				// find the tx with the same tx
				if let Some(tx) = txs.and_then(|txs| txs.into_iter().find(|tx| tx.hash == hash)) {
					return Ok(Some(tx))
				} else {
					// otherwise look into previous block
					latest = BlockId::Hash(header.parent_hash);
				}
			} else {
				break
			}
		}

		Ok(None)
	}

	pub fn get_transaction_from_pool(&self, hash: H256) -> Result<Option<Transaction>> {
		let mut xts: Vec<<B as BlockT>::Extrinsic> = Vec::new();

		xts.extend(
			self.graph
				.validated_pool()
				.ready()
				.map(|in_pool_tx| in_pool_tx.data().clone())
				.collect::<Vec<<B as BlockT>::Extrinsic>>(),
		);

		xts.extend(
			self.graph
				.validated_pool()
				.futures()
				.iter()
				.map(|(_hash, extrinsic)| extrinsic.clone())
				.collect::<Vec<<B as BlockT>::Extrinsic>>(),
		);

		let id: BlockId<B> = BlockId::Hash(self.client.info().best_hash);

		let filtered = self
			.client
			.runtime_api()
			.extrinsic_filter(&id, xts)
			.map_err(|e| internal_err(format!("unable to get filtered txs {e:?}")))?;

		let builder = block_builder::BlockBuilder::<B, C, A>::from_client(self.client.clone());

		Ok(filtered
			.iter()
			.find(|xt| xt.hash() == hash)
			.map(|tx| builder.expand_eth_transaction(tx, None, None, None)))
	}

	pub fn get_block_receipts(&self, block_number: Option<BlockNumber>) -> Result<Vec<Receipt>> {
		let builder = block_builder::BlockBuilder::<B, C, A>::from_client(self.client.clone());

		let rich_block = builder.to_rich_block(block_number, true)?;
		let receipts = builder.receipts(block_number)?;
		let statuses = builder.statuses(block_number)?;

		let txs = if let BlockTransactions::Full(txs) = &rich_block.transactions {
			Ok(txs.clone())
		} else {
			Err(internal_err("unable to get full transaction_status"))
		}?;

		let mut block_log_idx = 0;
		let mut rich_receipts = vec![];

		for (tx, (status, receipt)) in
			txs.into_iter().zip(statuses.into_iter().zip(receipts.into_iter()))
		{
			let r = match receipt {
				EthereumReceipt::Legacy(t) |
				EthereumReceipt::EIP1559(t) |
				EthereumReceipt::EIP2930(t) => t,
			};

			let r = Receipt {
				transaction_hash: Some(tx.hash),
				transaction_index: tx.transaction_index,
				block_hash: tx.block_hash,
				from: Some(tx.from),
				to: tx.to,
				block_number: tx.block_number,
				cumulative_gas_used: Default::default(),
				gas_used: Some(r.used_gas),
				contract_address: tx.creates,
				logs: status
					.logs
					.iter()
					.enumerate()
					.map(|(tx_log_idx, l)| Log {
						address: l.address,
						transaction_hash: Some(tx.hash),
						transaction_index: tx.transaction_index,
						block_hash: tx.block_hash,
						block_number: tx.block_number,
						data: l.data.clone().into(),
						transaction_log_index: Some(tx_log_idx.into()),
						topics: l.topics.clone(),
						log_index: Some((block_log_idx + tx_log_idx).into()),
						removed: false,
					})
					.collect(),
				state_root: Some(rich_block.header.state_root),
				logs_bloom: status.logs_bloom,
				status_code: Some(r.status_code.into()),
				effective_gas_price: Default::default(),
				transaction_type: tx
					.transaction_type
					.ok_or_else(|| internal_err("transaction_type not specified"))?,
			};

			block_log_idx += r.logs.len();

			rich_receipts.push(r);
		}

		Ok(rich_receipts)
	}

	pub fn get_transaction_receipt(&self, tx: Transaction) -> Result<Receipt> {
		let bn = tx.block_hash.map(|h| BlockNumber::Hash { hash: h, require_canonical: false });

		let receipts = self.get_block_receipts(bn)?;

		if let Some(receipt) = receipts.into_iter().find(|r| r.transaction_hash == Some(tx.hash)) {
			Ok(receipt)
		} else {
			Err(internal_err("fetch tx receipt failed"))
		}
	}
}
