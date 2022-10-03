#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_core::{ecdsa, H160};
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
	}
}
