//! block helper
//!
//! helper functions to respond to queries expecting eth_style richblock

use super::BlockMapper;
use crate::rpc::evm_rpc_compat::internal_err;
use codec::{Decode, Encode};
use ethereum::{TransactionAction, TransactionV2};
use fc_rpc_core::types::{
	Block, BlockNumber, BlockTransactions, Bytes, Header as EthHeader, Rich, RichBlock, Transaction,
};
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::Header;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_core::{H256, U256};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

pub struct BlockBuilder<B, C, A: ChainApi> {
	client: Arc<C>,
	graph: Arc<Pool<A>>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> BlockBuilder<B, C, A>
where
	B: BlockT<Header = Header>,
	A: ChainApi,
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B> + Sync + Send + 'static,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B>,
{
	pub fn from_client(client: Arc<C>, graph: Arc<Pool<A>>) -> Self {
		Self { client, graph, _marker: Default::default() }
	}

	pub(crate) fn build_eth_header(&self, header: <B as BlockT>::Header) -> EthHeader {
		// FIXME: fulfill all required fields
		EthHeader {
			hash: Some(header.hash()),
			parent_hash: *header.parent_hash(),
			uncles_hash: H256::default(),
			author: Default::default(),
			miner: Default::default(),
			state_root: *header.state_root(),
			transactions_root: *header.extrinsics_root(),
			receipts_root: Default::default(),
			number: Some(U256::from(*header.number())),
			gas_used: Default::default(),
			gas_limit: Default::default(),
			extra_data: Bytes(header.digest().encode()),
			logs_bloom: Default::default(),
			timestamp: Default::default(),
			difficulty: Default::default(),
			nonce: None,
			size: Some(U256::from(header.encode().len() as u32)),
		}
	}

	pub fn expand_eth_transaction(&self, tx: TransactionV2) -> Transaction {
		let mut eth_tx = Transaction::from(tx.clone());

		match tx {
			TransactionV2::Legacy(t) =>
				if let TransactionAction::Call(target) = t.action {
					eth_tx.to.replace(target);
				},
			TransactionV2::EIP2930(t) =>
				if let TransactionAction::Call(target) = t.action {
					eth_tx.to.replace(target);
				},
			TransactionV2::EIP1559(t) =>
				if let TransactionAction::Call(target) = t.action {
					eth_tx.to.replace(target);
				},
		};

		eth_tx
	}

	pub(crate) fn build_eth_transaction_status(
		&self,
		header: <B as BlockT>::Header,
		body: Vec<<B as BlockT>::Extrinsic>,
		full: bool,
	) -> BlockTransactions {
		// BlockTransactions;

		let block_number = U256::from(*header.number());
		let block_hash = header.hash();

		let eth_txs = body
			.iter()
			.filter_map(|raw_tx| {
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
			});

		if full {
			let txs = eth_txs
				.enumerate()
				.map(|(idx, t)| {
					// FIXME: replace other needed information
					let idx = U256::from(idx);
					let mut out = self.expand_eth_transaction(t);

					out.transaction_index.replace(idx);
					out.block_number.replace(block_number);
					out.block_hash.replace(block_hash);

					out
				})
				.collect();

			BlockTransactions::Full(txs)
		} else {
			BlockTransactions::Hashes(eth_txs.map(|v| v.hash()).collect())
		}
	}

	pub(crate) fn to_rich_block(
		&self,
		number: Option<BlockNumber>,
		full: bool,
	) -> Result<RichBlock> {
		let mapper = BlockMapper::from_client(self.client.clone(), self.graph.clone());

		let id = mapper
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let block = self.client.block(&id);
		let body = self.client.block_body(&id);
		let header = self.client.header(id);

		match (block, header, body) {
			(Ok(Some(block)), Ok(Some(header)), Ok(Some(body))) => {
				let rb = Rich {
					inner: Block {
						header: self.build_eth_header(header.clone()),
						total_difficulty: U256::zero(),
						uncles: vec![],
						transactions: self.build_eth_transaction_status(header, body, full),
						// TODO: this value is encoded using scale-codec, eth client expects output
						// of rlp?
						size: Some(U256::from(block.block.encode().len() as u32)),
						base_fee_per_gas: Default::default(),
					},
					extra_info: BTreeMap::new(),
				};

				Ok(rb)
			},
			_ => Err(internal_err("unable to gather required information to build rich_block")),
		}
	}
}
