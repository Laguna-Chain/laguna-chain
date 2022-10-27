//! transaction helper
//!
//! helper functions for transaction details

use super::{block_builder, internal_err};
use fc_rpc_core::types::{BlockNumber, BlockTransactions, Index, Receipt, Transaction};
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
		let builder =
			block_builder::BlockBuilder::from_client(self.client.clone(), self.graph.clone());
		let rich_block = builder
			.to_rich_block(Some(BlockNumber::Hash { hash, require_canonical: false }), true)?;

		if let BlockTransactions::Full(txs) = &rich_block.transactions {
			Ok(txs.iter().find(|v| v.transaction_index == Some(index.value().into())).cloned())
		} else {
			Ok(None)
		}
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

		tx.transaction_index
			.and_then(|i| {
				receipts.iter().enumerate().find_map(|(idx, r)| {
					if idx == i.as_usize() {
						Some(r)
					} else {
						None
					}
				})
			})
			.map(|r| Receipt {
				transaction_hash: Some(tx.hash),
				transaction_index: tx.transaction_index,
				block_hash: tx.block_hash,
				from: Some(tx.from),
				to: tx.to,
				block_number: tx.block_number,
				cumulative_gas_used: Default::default(),
				gas_used: None,
				contract_address: None,
				logs: vec![],
				state_root: None,
				logs_bloom: Default::default(),
				status_code: Some(r.status_code.into()),
				effective_gas_price: Default::default(),
				transaction_type: tx.transaction_type.unwrap_or_default(),
			})
			.ok_or_else(|| internal_err("fetch tx receipt failed"))
	}
}
