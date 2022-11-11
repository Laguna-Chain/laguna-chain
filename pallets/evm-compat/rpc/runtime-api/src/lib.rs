//! evm-compact-runtime-api
//!
//! This module contains various helper function for the client to work properly

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use ethereum::{
	BlockV2 as EthereumBlock, ReceiptV3 as EthereumReceipt, TransactionV2 as EthereumTransaction,
};
use fp_rpc::TransactionStatus;

use sp_core::{H160, H256, U256};
use sp_runtime::{traits::Block as BlockT, DispatchError};
use sp_std::vec::Vec;

pub type ConesensusDigest = ([u8; 4], Vec<u8>);

sp_api::decl_runtime_apis! {
	pub trait EvmCompatApi<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Codec,
	{

		/// find the mapped AccoundId
		fn source_to_mapped_address(source: H160) -> AccountId;

		fn source_to_contract_address(source: H160) -> AccountId;

		fn check_source_is_contract(source: H160) -> bool;

		/// check whether this h160 has a backing proxy behind it
		fn source_is_backed_by(source: H160) -> Option<AccountId>;

		fn check_contract_is_evm_compat(contract_addr: AccountId) -> Option<H160>;

		/// get chain_id
		fn chain_id() -> u64;

		/// balances of the h160 address, only returns accounts not contracts
		fn balances(address: H160) -> U256;

		/// read contract storage of a contract
		fn storage_at(address: H160, index: U256,) -> H256;

		/// nonce of the address
		fn account_nonce(addrss: H160) -> U256;

		/// try-run a transaction, used to get the estimated cost or return value
		fn call(from: Option<H160>, target: Option<H160>, value: Balance, input: Vec<u8>, gas_limit: U256,  gas_price: U256) ->  Result<(Vec<u8>, Balance), DispatchError>;

		/// return only extrinsics that contains valid eth-transaction
		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction>;

		fn map_block(block: Block) -> EthereumBlock;

		fn transaction_status(block: Block) -> Vec<TransactionStatus>;

		fn transaction_receipts(block: Block) -> Vec<EthereumReceipt>;
	}
}
