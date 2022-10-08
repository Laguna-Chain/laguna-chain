//! transaction helper
//!
//! helper functions for transaction details

use codec::{Decode, Encode};
use fc_rpc_core::types::{BlockNumber, BlockTransactions, Index, Receipt, Transaction};
use fp_rpc::ConvertTransactionRuntimeApi;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi as EvmCompatRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{Backend, BlockBackend, HeaderBackend, StateBackend, StorageProvider};
use sc_network::ExHashT;
use sc_service::InPoolTransaction;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::H256;
use sp_runtime::{
	generic::BlockId,
	traits::{BlakeTwo256, Block as BlockT},
};

use super::EthApi;

impl<B, C, H: ExHashT, CT, BE, P, A> EthApi<B, C, H, CT, BE, P, A>
where
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C::Api: ConvertTransactionRuntimeApi<B>,
	C::Api: EvmCompatRuntimeApi<B, AccountId, Balance>,
	C::Api: BlockBuilderApi<B>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	CT: fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
	P: TransactionPool<Block = B> + Send + Sync + 'static,
	A: ChainApi<Block = B> + 'static,
{
	pub fn trasnaction_recepit(&self, tx: Transaction) -> Receipt {
		Receipt {
			transaction_hash: Some(tx.hash),
			transaction_index: tx.transaction_index,
			block_hash: tx.block_hash,
			from: Some(tx.from),
			to: tx.to,
			block_number: tx.block_number,
			cumulative_gas_used: Default::default(),
			gas_used: Default::default(),
			contract_address: None,
			logs: vec![],
			state_root: None,
			logs_bloom: Default::default(),
			status_code: None,
			effective_gas_price: Default::default(),
			transaction_type: tx.transaction_type.unwrap_or_default(),
		}
	}

	pub async fn get_transaction_by_block_number_and_index(
		&self,
		number: BlockNumber,
		index: Index,
	) -> Result<Option<Transaction>> {
		let rich_block = self.to_rich_block(Some(number), true)?;

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
		let rich_block =
			self.to_rich_block(Some(BlockNumber::Hash { hash, require_canonical: false }), true)?;

		if let BlockTransactions::Full(txs) = &rich_block.transactions {
			Ok(txs.iter().find(|v| v.transaction_index == Some(index.value().into())).cloned())
		} else {
			Ok(None)
		}
	}

	pub fn get_transactions(
		&self,
		number: Option<BlockNumber>,
	) -> Result<Option<Vec<Transaction>>> {
		let rich_block = self.to_rich_block(number, true)?;

		if let BlockTransactions::Full(txs) = &rich_block.transactions {
			Ok(Some(txs.clone()))
		} else {
			Ok(None)
		}
	}

	pub fn get_transaction_from_pool(&self, hash: H256) -> Result<Option<Transaction>> {
		Ok(self
			.graph
			.validated_pool()
			.ready()
			.map(|in_pool_tx| in_pool_tx.data().clone())
			.filter_map(|raw_tx: UncheckedExtrinsic| {
				laguna_runtime::UncheckedExtrinsic::decode(&mut &raw_tx.encode()[..]).ok()
			})
			.filter_map(|xt| {
				if let laguna_runtime::Call::EvmCompat(pallet_evm_compat::Call::transact { t }) =
					xt.0.function
				{
					Some(t)
				} else {
					None
				}
			})
			.find(|tx| tx.hash() == hash)
			.map(|v| self.expand_eth_transaction(v)))
	}

	pub fn get_transaction_from_blocks(&self, hash: H256) -> Result<Option<Transaction>> {
		let mut latest = BlockId::<B>::hash(self.client.info().best_hash);

		while let Ok(Some(header)) = self.client.header(latest) {
			let txs_rs = self.get_transactions(Some(BlockNumber::Hash {
				require_canonical: false,
				hash: header.hash(),
			}))?;

			if let Some(tx) = txs_rs.and_then(|txs| txs.into_iter().find(|tx| tx.hash == hash)) {
				return Ok(Some(tx))
			} else {
				latest = BlockId::Hash(header.parent_hash);
			}
		}

		Ok(None)
	}
}
