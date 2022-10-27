//! block helper
//!
//! helper functions to respond to queries expecting eth_style richblock
use super::{deferrable_runtime_api::DeferrableApi, BlockMapper};
use crate::rpc::evm_rpc_compat::internal_err;
use codec::Encode;
use ethereum::{BlockV2 as EthereumBlock, EIP658ReceiptData, PartialHeader, TransactionV2};
use fc_rpc::public_key;
use fc_rpc_core::types::{
	Block, BlockNumber, BlockTransactions, Bytes, Header as EthHeader, Rich, RichBlock, Transaction,
};
use fp_rpc::TransactionStatus;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{keccak_256, H160, H256, H512, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

pub struct BlockBuilder<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> BlockBuilder<B, C, A>
where
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic>,
	A: ChainApi<Block = B> + 'static + Sync + Send,
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	pub fn from_client(client: Arc<C>, graph: Arc<Pool<A>>) -> Self {
		Self { client, graph, _marker: Default::default() }
	}

	// derived from frontier, turns payload tx into block tx format
	pub fn expand_eth_transaction(
		&self,
		ethereum_transaction: &TransactionV2,
		block: Option<&EthereumBlock>,
		status: Option<TransactionStatus>,
		base_fee: Option<U256>,
	) -> Transaction {
		let mut transaction: Transaction = Transaction::from(ethereum_transaction.clone());

		if let TransactionV2::EIP1559(_) = ethereum_transaction {
			if block.is_none() && status.is_none() {
				// If transaction is not mined yet, gas price is considered just max fee per gas.
				transaction.gas_price = transaction.max_fee_per_gas;
			} else {
				let base_fee = base_fee.unwrap_or_default();
				let max_priority_fee_per_gas =
					transaction.max_priority_fee_per_gas.unwrap_or_default();
				let max_fee_per_gas = transaction.max_fee_per_gas.unwrap_or_default();
				// If transaction is already mined, gas price is the effective gas price.
				transaction.gas_price = Some(
					base_fee
						.checked_add(max_priority_fee_per_gas)
						.unwrap_or_else(U256::max_value)
						.min(max_fee_per_gas),
				);
			}
		}

		let pubkey = match public_key(ethereum_transaction) {
			Ok(p) => Some(p),
			Err(_e) => None,
		};

		let block_hash = block.as_ref().and_then(|block| {
			self.client
				.header(BlockId::Number(block.header.number.as_u32()))
				.ok()
				.and_then(|h| h.map(|h| h.hash()))
		});

		// Block hash.
		transaction.block_hash = block_hash;
		// Block number.
		transaction.block_number = block.as_ref().map(|block| block.header.number);
		// Transaction index.
		transaction.transaction_index = status.as_ref().map(|status| {
			U256::from(UniqueSaturatedInto::<u32>::unique_saturated_into(status.transaction_index))
		});
		// From.
		transaction.from = status.as_ref().map_or(
			{
				match pubkey {
					Some(pk) => H160::from(H256::from(keccak_256(&pk))),
					_ => H160::default(),
				}
			},
			|status| status.from,
		);
		// To.
		transaction.to = status.as_ref().map_or(
			{
				let action = match ethereum_transaction {
					TransactionV2::Legacy(t) => t.action,
					TransactionV2::EIP2930(t) => t.action,
					TransactionV2::EIP1559(t) => t.action,
				};
				match action {
					ethereum::TransactionAction::Call(to) => Some(to),
					_ => None,
				}
			},
			|status| status.to,
		);
		// Creates.
		transaction.creates = status.as_ref().and_then(|status| status.contract_address);
		// Public key.
		transaction.public_key = pubkey.as_ref().map(H512::from);

		transaction
	}

	pub(crate) fn to_eth_block(&self, number: Option<BlockNumber>) -> Result<EthereumBlock> {
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper.map_block(number);
		let id = id.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let res = self
			.client
			.block(&id)
			.map_err(|e| internal_err(format!("unable to get block {e:?}")))?;

		let block = res.map(|b| b.block).ok_or_else(|| internal_err("unable to get block"))?;

		self.client
			.runtime_api()
			.map_block(&id, block)
			.map_err(|e| internal_err(format!("unable to map eth_block {e:?}")))
	}

	pub(crate) fn statuses(&self, number: Option<BlockNumber>) -> Result<Vec<TransactionStatus>> {
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper.map_block(number);
		let id = id.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let res = self
			.client
			.block(&id)
			.map_err(|e| internal_err(format!("unable to get block {e:?}")))?;

		let block = res.map(|b| b.block).ok_or_else(|| internal_err("unable to get block"))?;

		self.client
			.runtime_api()
			.transaction_status(&id, block)
			.map_err(|e| internal_err(format!("unable to get statuses {e:?}")))
	}

	pub(crate) fn receipts(&self, number: Option<BlockNumber>) -> Result<Vec<EIP658ReceiptData>> {
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper.map_block(number);
		let id = id.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let res = self
			.client
			.block(&id)
			.map_err(|e| internal_err(format!("unable to get block {e:?}")))?;

		let block = res.map(|b| b.block).ok_or_else(|| internal_err("unable to get block"))?;

		self.client
			.runtime_api()
			.transaction_receipts(&id, block)
			.map_err(|e| internal_err(format!("unable to get receipts {e:?}")))
	}

	pub fn build_eth_statuses(
		&self,
		number: Option<BlockNumber>,
		full: bool,
	) -> Result<BlockTransactions> {
		let block = self.to_eth_block(number)?;
		let txs = &block.transactions;

		let statuses = self.statuses(number)?;

		let status = if full {
			let expanded = txs
				.iter()
				.zip(statuses.into_iter())
				.map(|(tx, status)| {
					self.expand_eth_transaction(tx, Some(&block), Some(status), None)
				})
				.collect::<Vec<_>>();
			BlockTransactions::Full(expanded)
		} else {
			BlockTransactions::Hashes(statuses.iter().map(|tx| tx.transaction_hash).collect())
		};

		Ok(status)
	}

	pub(crate) fn to_rich_block(
		&self,
		number: Option<BlockNumber>,
		full: bool,
	) -> Result<RichBlock> {
		let block = self.to_eth_block(number)?;

		let tx_statuses = self.build_eth_statuses(number, full)?;
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let header = self
			.client
			.header(id)
			.map_err(|e| internal_err(format!("unable to obtain block header {e:?}")))?;

		let rb = Rich {
			inner: Block {
				header: EthHeader {
					// bypass substrate block_header as eth_block header
					hash: header.map(|h| h.hash()),
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
					timestamp: U256::from(block.header.timestamp / 1000),
					difficulty: block.header.difficulty,
					nonce: Some(block.header.nonce),
					size: Some(U256::from(rlp::encode(&block.header).len() as u32)),
				},
				total_difficulty: U256::zero(),
				uncles: vec![],
				transactions: tx_statuses,
				size: Some(U256::from(rlp::encode(&block).len() as u32)),
				base_fee_per_gas: None,
			},
			extra_info: BTreeMap::new(),
		};

		Ok(rb)
	}
}
