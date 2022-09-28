use ethereum_types::{H64, U64};
use fc_rpc_core::{
	types::{
		BlockNumber, Bytes, CallRequest, FeeHistory, Index, PeerCount, Receipt, RichBlock,
		SyncStatus, Transaction, TransactionRequest, Work,
	},
	EthApiServer, EthPubSubApiServer, NetApiServer,
};
use jsonrpsee::core::{async_trait, Error as CoreError, RpcResult as Result};
use sc_client_api::HeaderBackend;
use sc_network::{ExHashT, NetworkService};
use sp_api::ProvideRuntimeApi;
use sp_core::{H160, H256, U256};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::{marker::PhantomData, sync::Arc};

pub struct Net<B: BlockT, C, H: ExHashT> {
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	peer_count_as_hex: bool,
}

impl<B: BlockT, C, H: ExHashT> Net<B, C, H> {
	pub fn new(
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		peer_count_as_hex: bool,
	) -> Self {
		Self { client, network, peer_count_as_hex }
	}
}

pub fn err<T: ToString>(code: i32, message: T, data: Option<&[u8]>) -> jsonrpsee::core::Error {
	jsonrpsee::core::Error::Call(jsonrpsee::types::error::CallError::Custom(
		jsonrpsee::types::error::ErrorObject::owned(
			code,
			message.to_string(),
			data.map(|bytes| {
				jsonrpsee::core::to_json_raw_value(&format!("0x{}", hex::encode(bytes)))
					.expect("fail to serialize data")
			}),
		),
	))
}

pub fn internal_err<T: ToString>(message: T) -> jsonrpsee::core::Error {
	err(jsonrpsee::types::error::INTERNAL_ERROR_CODE, message, None)
}

impl<B, C, H: ExHashT> NetApiServer for Net<B, C, H>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
{
	fn version(&self) -> Result<String> {
		Ok(format!("0x{:x}", 1000))
	}

	fn peer_count(&self) -> Result<PeerCount> {
		let peer_count = self.network.num_connected();
		Ok(match self.peer_count_as_hex {
			true => PeerCount::String(format!("0x{:x}", peer_count)),
			false => PeerCount::U32(peer_count as u32),
		})
	}

	fn is_listening(&self) -> Result<bool> {
		Ok(true)
	}
}

pub struct EthApi<B: BlockT, C, H: ExHashT> {
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	peer_count_as_hex: bool,
}

impl<B: BlockT, C, H: ExHashT> EthApi<B, C, H> {
	pub fn new(
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		peer_count_as_hex: bool,
	) -> Self {
		Self { client, network, peer_count_as_hex }
	}
}

#[async_trait]
impl<B, C, H: ExHashT> EthApiServer for EthApi<B, C, H>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
{
	// ########################################################################
	// Client
	// ########################################################################

	/// Returns protocol version encoded as a string (quotes are necessary).
	fn protocol_version(&self) -> Result<u64> {
		todo!()
	}

	/// Returns an object with data about the sync status or false. (wtf?)
	fn syncing(&self) -> Result<SyncStatus> {
		todo!()
	}

	/// Returns block author.
	fn author(&self) -> Result<H160> {
		todo!()
	}

	/// Returns accounts list.
	fn accounts(&self) -> Result<Vec<H160>> {
		todo!()
	}

	/// Returns highest block number.
	fn block_number(&self) -> Result<U256> {
		todo!()
	}

	/// Returns the chain ID used for transaction signing at the
	/// current best block. None is returned if not
	/// available.
	fn chain_id(&self) -> Result<Option<U64>> {
		todo!()
	}

	// ########################################################################
	// Block
	// ########################################################################

	/// Returns block with given hash.
	async fn block_by_hash(&self, hash: H256, full: bool) -> Result<Option<RichBlock>> {
		todo!()
	}

	/// Returns block with given number.
	async fn block_by_number(&self, number: BlockNumber, full: bool) -> Result<Option<RichBlock>> {
		todo!()
	}

	/// Returns the number of transactions in a block with given hash.
	fn block_transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
		todo!()
	}

	/// Returns the number of transactions in a block with given block number.
	fn block_transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>> {
		todo!()
	}

	/// Returns the number of uncles in a block with given hash.
	fn block_uncles_count_by_hash(&self, hash: H256) -> Result<U256> {
		todo!()
	}

	/// Returns the number of uncles in a block with given block number.
	fn block_uncles_count_by_number(&self, number: BlockNumber) -> Result<U256> {
		todo!()
	}

	/// Returns an uncles at given block and index.
	fn uncle_by_block_hash_and_index(&self, hash: H256, index: Index) -> Result<Option<RichBlock>> {
		todo!()
	}

	/// Returns an uncles at given block and index.
	fn uncle_by_block_number_and_index(
		&self,
		number: BlockNumber,
		index: Index,
	) -> Result<Option<RichBlock>> {
		todo!()
	}

	// ########################################################################
	// Transaction
	// ########################################################################

	/// Get transaction by its hash.
	async fn transaction_by_hash(&self, hash: H256) -> Result<Option<Transaction>> {
		todo!()
	}

	/// Returns transaction at given block hash and index.
	async fn transaction_by_block_hash_and_index(
		&self,
		hash: H256,
		index: Index,
	) -> Result<Option<Transaction>> {
		todo!()
	}

	/// Returns transaction by given block number and index.
	async fn transaction_by_block_number_and_index(
		&self,
		number: BlockNumber,
		index: Index,
	) -> Result<Option<Transaction>> {
		todo!()
	}

	/// Returns transaction receipt by transaction hash.
	async fn transaction_receipt(&self, hash: H256) -> Result<Option<Receipt>> {
		todo!()
	}

	// ########################################################################
	// State
	// ########################################################################

	/// Returns balance of the given account.
	fn balance(&self, address: H160, number: Option<BlockNumber>) -> Result<U256> {
		todo!()
	}

	/// Returns content of the storage at given address.
	fn storage_at(&self, address: H160, index: U256, number: Option<BlockNumber>) -> Result<H256> {
		todo!()
	}

	/// Returns the number of transactions sent from given address at given time (block number).
	fn transaction_count(&self, address: H160, number: Option<BlockNumber>) -> Result<U256> {
		todo!()
	}

	/// Returns the code at given address at given time (block number).
	fn code_at(&self, address: H160, number: Option<BlockNumber>) -> Result<Bytes> {
		todo!()
	}

	// ########################################################################
	// Execute
	// ########################################################################

	/// Call contract, returning the output data.
	fn call(&self, request: CallRequest, number: Option<BlockNumber>) -> Result<Bytes> {
		todo!()
	}

	/// Estimate gas needed for execution of given contract.
	async fn estimate_gas(
		&self,
		request: CallRequest,
		number: Option<BlockNumber>,
	) -> Result<U256> {
		todo!()
	}

	// ########################################################################
	// Fee
	// ########################################################################

	/// Returns current gas_price.
	fn gas_price(&self) -> Result<U256> {
		todo!()
	}

	/// Introduced in EIP-1159 for getting information on the appropriate priority fee to use.
	fn fee_history(
		&self,
		block_count: U256,
		newest_block: BlockNumber,
		reward_percentiles: Option<Vec<f64>>,
	) -> Result<FeeHistory> {
		todo!()
	}

	/// Introduced in EIP-1159, a Geth-specific and simplified priority fee oracle.
	/// Leverages the already existing fee history cache.
	fn max_priority_fee_per_gas(&self) -> Result<U256> {
		unimplemented!()
	}

	// ########################################################################
	// Mining
	// ########################################################################

	/// Returns true if client is actively mining new blocks.
	fn is_mining(&self) -> Result<bool> {
		unimplemented!()
	}

	/// Returns the number of hashes per second that the node is mining with.
	fn hashrate(&self) -> Result<U256> {
		unimplemented!()
	}

	/// Returns the hash of the current block, the seedHash, and the boundary condition to be met.
	fn work(&self) -> Result<Work> {
		unimplemented!()
	}

	/// Used for submitting mining hashrate.
	fn submit_hashrate(&self, hashrate: U256, id: H256) -> Result<bool> {
		unimplemented!()
	}

	/// Used for submitting a proof-of-work solution.
	fn submit_work(&self, nonce: H64, pow_hash: H256, mix_digest: H256) -> Result<bool> {
		unimplemented!()
	}

	// ########################################################################
	// Submit
	// ########################################################################

	/// Sends transaction; will block waiting for signer to return the
	/// transaction hash.
	async fn send_transaction(&self, request: TransactionRequest) -> Result<H256> {
		todo!()
	}

	/// Sends signed transaction, returning its hash.
	async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256> {
		todo!()
	}
}
