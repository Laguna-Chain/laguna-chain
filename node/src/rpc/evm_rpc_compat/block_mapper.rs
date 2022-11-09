use super::internal_err;
use ethereum::BlockV2 as EthereumBlock;
use fc_rpc_core::types::BlockNumber;
use jsonrpsee::core::RpcResult as Result;
use laguna_runtime::opaque::{Header, UncheckedExtrinsic};
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool::ChainApi;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, UniqueSaturatedInto},
};

use fc_rpc_core::types::{Filter, Log};

use tokio::time::{timeout, Duration};

use std::{marker::PhantomData, sync::Arc};

/// ethereum request block time to a greater extend, we can ansower some of them locally, lets try!
pub struct BlockMapper<B, C, A: ChainApi> {
	client: Arc<C>,
	_marker: PhantomData<(B, A)>,
}

impl<B, C, A> BlockMapper<B, C, A>
where
	A: ChainApi<Block = B> + 'static + Sync + Send,
	B: BlockT<Hash = H256, Header = Header, Extrinsic = UncheckedExtrinsic> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B>,
	C::Api: EvmCompatApiRuntimeApi<B, AccountId, Balance>,
	C: BlockBackend<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: BlockBuilderApi<B>,
{
	pub fn from_client(client: Arc<C>) -> BlockMapper<B, C, A> {
		BlockMapper { client, _marker: PhantomData }
	}

	pub(crate) fn map_block(&self, number: Option<BlockNumber>) -> Option<BlockId<B>> {
		let client = &self.client;

		match number.unwrap_or(BlockNumber::Latest) {
			BlockNumber::Hash { hash, .. } => Some(BlockId::<B>::Hash(hash)),
			BlockNumber::Num(number) => Some(BlockId::Number(number.unique_saturated_into())),
			BlockNumber::Latest => Some(BlockId::Hash(client.info().best_hash)),
			BlockNumber::Earliest => Some(BlockId::Hash(client.info().genesis_hash)),
			BlockNumber::Pending => None,
		}
	}

	pub fn reflect_block(&self, number: Option<BlockNumber>) -> Result<Option<EthereumBlock>> {
		let id = self
			.map_block(number)
			.unwrap_or_else(|| BlockId::Hash(self.client.info().best_hash));

		match self.client.block(&id) {
			Ok(Some(b)) => self
				.client
				.runtime_api()
				.map_block(&id, b.block)
				.map(Some)
				.map_err(|e| internal_err(format!("reflect eth block failed: {:?}", e))),
			Ok(None) => Ok(None),
			Err(e) => Err(internal_err(format!("fetch substrate block failed: {:?}", e))),
		}
	}

	pub fn find_author(&self, number: Option<BlockNumber>) -> Result<Option<H160>> {
		self.reflect_block(number).map(|v| v.map(|b| b.header.beneficiary))
	}

	pub fn transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
		let number = BlockNumber::Hash { hash, require_canonical: false };
		self.transaction_count_by_number(number)
	}

	pub fn transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>> {
		self.reflect_block(Some(number)).map(|v| v.map(|b| b.transactions.len().into()))
	}

	// get logs of the target block
	pub async fn block_logs(&self, block_hash: H256) -> Result<Vec<Log>> {
		let id = BlockId::<B>::Hash(block_hash);
		let s_bnumber = self.client.number(block_hash).unwrap().unwrap();

		let s_block = self.client.block(&id).unwrap().unwrap();
		let statuses = self.client.runtime_api().transaction_status(&id, s_block.block).unwrap();

		let mut logs = vec![];

		for s in statuses.iter() {
			let s_logs = s.logs.clone();

			for (l_idx, l) in s_logs.into_iter().enumerate() {
				let l = Log {
					address: l.address,
					topics: l.topics.clone(),
					data: fc_rpc_core::types::Bytes(l.data.clone()),
					block_hash: Some(block_hash),
					block_number: Some(s_bnumber.into()),
					transaction_hash: Some(s.transaction_hash),
					transaction_index: Some(s.transaction_index.into()),
					log_index: Some(logs.len().into()),
					transaction_log_index: Some(l_idx.into()),
					removed: false,
				};

				logs.push(l);
			}
		}

		Ok(logs)
	}

	pub(crate) async fn range_block_logs(&self, from: u32, to: u32) -> Result<Vec<Log>> {
		let mut current = from;
		let mut logs = vec![];

		while current <= to {
			let bhash = self
				.client
				.hash(from)
				.map_err(|e| internal_err(format!("unable to get block_hash: {:?}", e)))?
				.ok_or_else(|| internal_err("could got get blockhash"))?;

			let mut block_logs = self.block_logs(bhash).await?;
			logs.append(&mut block_logs);
			current += 1;
		}

		Ok(logs)
	}

	pub(crate) async fn logs(
		&self,
		Filter { block_hash, from_block, to_block, .. }: Filter,
	) -> Result<Vec<Log>> {
		// this is run in dumping mode, not other filers params will be taken seriously

		match (block_hash, from_block, to_block) {
			(Some(hash), _, _) => return self.block_logs(hash).await,
			(None, _, _) => {
				let best_number = self.client.info().best_number;

				let mut current_number = to_block
					.and_then(|v| v.to_min_block_num())
					.map(|s| s.unique_saturated_into())
					.unwrap_or(best_number);

				if current_number > best_number {
					current_number = best_number;
				}

				let from_number = from_block
					.and_then(|v| v.to_min_block_num())
					.map(|s| s.unique_saturated_into())
					.unwrap_or(best_number);

				// cut-off if things goes sideways
				timeout(Duration::from_secs(10), self.range_block_logs(from_number, current_number))
					.await
					.map_err(|e| internal_err(format!("timeout limit reached: {:?}", e)))?
			},
		}
	}
}
