//! deferrable helper
//!
//! ethereum request will often into either pending tx's or past blocks that might not be there for
//! a non-indexer node, this helper allows the runtime-api to apply tx's in the tx-pool manually and
//! answer the question

use crate::rpc::evm_rpc_compat::internal_err;
use codec::{Decode, Encode};
use ethereum::{TransactionAction, TransactionV2};
use fc_rpc_core::types::{
	Block, BlockNumber, BlockTransactions, Bytes, CallRequest, FeeHistory, Header as EthHeader,
	Rich, RichBlock, Transaction,
};
use fp_ethereum::TransactionData;
use fp_rpc::ConvertTransactionRuntimeApi;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi as EvmCompatRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{Backend, BlockBackend, HeaderBackend, StateBackend, StorageProvider};
use sc_network::ExHashT;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{crypto::UncheckedInto, keccak_256, H160, H256, U256};
use sp_runtime::{
	generic::{BlockId, Digest},
	traits::{BlakeTwo256, Block as BlockT},
};

use std::collections::BTreeMap;

use super::{pending_api::pending_runtime_api, BlockMapper, EthApi};

impl<B, C, H: ExHashT, CT, BE, P, A> EthApi<B, C, H, CT, BE, P, A>
where
	B: BlockT<Hash = H256, Extrinsic = UncheckedExtrinsic, Header = Header> + Send + Sync + 'static,
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
	pub fn find_digest(&self, at: &BlockId<B>) -> Result<Vec<([u8; 4], Vec<u8>)>> {
		let header = self.client.header(*at).map_err(|err| {
			internal_err(format!("fetch runtime header digest failed: {:?}", err))
		})?;

		header
			.ok_or_else(|| internal_err("fetch runtime header digest failed"))
			.map(|v| extract_digest(v.digest()))
	}

	pub fn find_author(&self, number: Option<BlockNumber>) -> Result<Option<H160>> {
		let mapper = BlockMapper::from_client(self.client.clone());

		let id = mapper
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		let latest_digests = self.find_digest(&id)?;

		self.client
			.runtime_api()
			.author(&id, latest_digests)
			.map_err(|err| internal_err(format!("fetch runtime author failed: {:?}", err)))
	}

	pub fn transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
		let number = BlockNumber::Hash { hash, require_canonical: false };
		self.transaction_count_by_number(number)
	}

	pub fn transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>> {
		let mapper = BlockMapper::from_client(self.client.clone());

		if let Some(id) = mapper.map_block(Some(number)) {
			// Get all transactions from the target block.
			self.client
				.block_body(&id)
				.map(|v| v.map(|o| U256::from(o.len())))
				.map_err(|err| internal_err(format!("fetch runtime block body failed: {:?}", err)))
		} else {
			// Get all transactions in the ready queue.
			let len = self.graph.validated_pool().ready().count();
			Ok(Some(U256::from(len)))
		}
	}

	pub(crate) fn build_eth_header(&self, header: <B as BlockT>::Header) -> EthHeader {
		// FIXME: fulfill all required fields
		EthHeader {
			hash: Some(header.hash()),
			parent_hash: *header.parent_hash(),
			uncles_hash: H256::default(),
			author: Default::default(),
			miner: Default::default(),
			state_root: header.state_root,
			transactions_root: header.extrinsics_root,
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
			.filter_map(|raw_tx: &UncheckedExtrinsic| {
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
		let mapper = BlockMapper::from_client(self.client.clone());

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

fn extract_digest(digest: &Digest) -> Vec<([u8; 4], Vec<u8>)> {
	digest
		.logs()
		.iter()
		.filter_map(|v| v.as_consensus().map(|(a, b)| (a, b.to_vec())))
		.collect::<Vec<_>>()
}
