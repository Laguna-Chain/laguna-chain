#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use pallet_contracts_primitives::ExecReturnValue;
use sp_core::{H160, H256, U256};
use sp_runtime::DispatchError;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait EvmCompatApi<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Codec,
	{
		fn source_to_mapped_address(source: H160) -> AccountId;

		fn source_is_backed_by(source: H160) -> Option<AccountId>;

		fn check_contract_is_evm_compat(contract_addr: AccountId) -> Option<H160>;

		fn chain_id() -> u64;

		fn balances(address: H160) -> U256;


		fn block_hash(number: u32) -> H256;

		fn storage_at(address: H160, index: U256,) -> H256;

		fn account_nonce(addrss: H160) -> U256;

		fn call(from: Option<H160>, target: Option<H160>, value: Balance, input: Vec<u8>, gas_limit: u64, storage_deposit_limit: Option<Balance>) ->  Result<(u64, ExecReturnValue), DispatchError>;
	}
}
