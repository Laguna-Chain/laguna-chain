//! block helper
//!
//! helper functions to respond to queries expecting eth_style richblock
use super::BlockMapper;
use crate::rpc::evm_rpc_compat::internal_err;
use codec::{Decode, Encode};
use ethereum::{BlockV2 as EthereumBlock, PartialHeader, TransactionAction, TransactionV2};
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
use sc_service::InPoolTransaction;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_core::{keccak_256, H160, H256, H512, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

type BlockTx<Block> = Vec<<Block as BlockT>::Extrinsic>;

pub struct BlockBuilder<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> BlockBuilder<B, C, A>
where
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic>,
	A: ChainApi<Block = B>,
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
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
		let mut transaction: Transaction = ethereum_transaction.clone().into();

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

		// Block hash.
		transaction.block_hash = block.as_ref().map(|block| block.header.hash());
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

	pub(crate) fn build_eth_transactions(
		&self,
		header: &<B as BlockT>::Header,
		body: Vec<<B as BlockT>::Extrinsic>,
	) -> Vec<TransactionV2> {
		// BlockTransactions;
		let block_hash = header.hash();

		self.client
			.runtime_api()
			.extrinsic_filter(&BlockId::Hash(block_hash), body)
			.unwrap_or_default()
	}

	pub(crate) fn to_eth_block(&self, number: Option<BlockNumber>) -> Result<EthereumBlock> {
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let body = self.client.block_body(&id);
		let header = self.client.header(id);

		match (header, body) {
			(Ok(Some(header)), Ok(Some(body))) => {
				let txs = self.build_eth_transactions(&header, body);

				let p_header = PartialHeader {
					parent_hash: *header.parent_hash(),
					beneficiary: Default::default(),
					state_root: *header.state_root(),
					receipts_root: Default::default(),
					number: U256::from(*header.number()),
					gas_used: Default::default(),
					gas_limit: Default::default(),
					extra_data: header.digest().encode(),
					logs_bloom: Default::default(),
					timestamp: Default::default(),
					difficulty: Default::default(),
					nonce: Default::default(),
					mix_hash: Default::default(),
				};

				let eth_block = EthereumBlock::new(p_header, txs, vec![]);

				Ok(eth_block)
			},
			_ => Err(internal_err("unable to gather required information to build rich_block")),
		}
	}

	pub fn empty_statuses(txs: &[TransactionV2]) -> Vec<TransactionStatus> {
		txs.iter()
			.enumerate()
			.map(|(idx, tx)| TransactionStatus {
				transaction_index: idx as _,
				transaction_hash: tx.hash(),
				..Default::default()
			})
			.collect()
	}

	pub fn build_eth_statuses(
		&self,
		number: Option<BlockNumber>,
		full: bool,
	) -> Result<BlockTransactions> {
		let block = self.to_eth_block(number)?;
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let body = self.client.block_body(&id);
		let header = self.client.header(id);

		match (header, body) {
			(Ok(Some(header)), Ok(Some(body))) => {
				let txs = self.build_eth_transactions(&header, body);

				let status = if full {
					let expanded = txs
						.iter()
						.zip(Self::empty_statuses(&txs[..]))
						.map(|(tx, s)| self.expand_eth_transaction(tx, Some(&block), Some(s), None))
						.collect::<Vec<_>>();
					BlockTransactions::Full(expanded)
				} else {
					BlockTransactions::Hashes(txs.iter().map(|tx| tx.hash()).collect())
				};

				Ok(status)
			},
			_ => Err(internal_err("unable to gather required information to build block_statuses")),
		}
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
