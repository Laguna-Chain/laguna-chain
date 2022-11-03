//! transaction helper
//!
//! helper functions for transaction details

use super::{block_builder, internal_err};
use ethereum::TransactionV2 as EthereumTransaction;
use fc_rpc_core::types::{BlockNumber, BlockTransactions, Index, Log, Receipt, Transaction};
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
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
		let builder =
			block_builder::BlockBuilder::from_client(self.client.clone(), self.graph.clone());

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
		let builder =
			block_builder::BlockBuilder::from_client(self.client.clone(), self.graph.clone());

		// starting from latest

		// checking previous block until we can't find any block
		while let Ok(Some(header)) = self.client.header(latest) {
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
		}

		Ok(None)
	}

	pub fn get_transaction_receipt(&self, tx: Transaction) -> Result<Receipt> {
		let builder =
			block_builder::BlockBuilder::from_client(self.client.clone(), self.graph.clone());

		let receipts = builder.receipts(Some(BlockNumber::Hash {
			hash: tx.block_hash.unwrap_or_default(),
			require_canonical: false,
		}))?;

		let statuses = builder.statuses(Some(BlockNumber::Hash {
			hash: tx.block_hash.unwrap_or_default(),
			require_canonical: false,
		}))?;

		// all consumed before the current transaction_index
		let cumulated = receipts
			.iter()
			.enumerate()
			.filter_map(|(idx, item)| {
				if tx.transaction_index.map(|v| v < idx.into()).unwrap_or(false) {
					Some(item.used_gas)
				} else {
					None
				}
			})
			.reduce(|acc, item| acc + item)
			.unwrap_or_default();

		receipts
			.iter()
			.zip(statuses.iter())
			.find_map(|(r, s)| {
				if let Some(i) = tx.transaction_index {
					if s.transaction_index == i.as_u32() {
						return Some((r, s))
					}
				}
				None
			})
			.map(|(r, s)| Receipt {
				transaction_hash: Some(tx.hash),
				transaction_index: tx.transaction_index,
				block_hash: tx.block_hash,
				from: Some(tx.from),
				to: tx.to,
				block_number: tx.block_number,
				cumulative_gas_used: cumulated,
				gas_used: Some(r.used_gas),
				contract_address: tx.creates,
				logs: s
					.logs
					.iter()
					.map(|l| Log {
						address: l.address,
						transaction_hash: Some(tx.hash),
						transaction_index: tx.transaction_index,
						block_hash: tx.block_hash,
						block_number: tx.block_number,
						data: l.data.clone().into(),
						log_index: None,
						topics: l.topics.clone(),
						transaction_log_index: None,
						removed: false,
					})
					.collect(),
				state_root: None,
				logs_bloom: s.logs_bloom,
				status_code: Some(r.status_code.into()),
				effective_gas_price: Default::default(),
				transaction_type: tx.transaction_type.unwrap_or_default(),
			})
			.ok_or_else(|| internal_err("fetch tx receipt failed"))
	}
}
