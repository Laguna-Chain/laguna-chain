#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use primitives::CurrencyId;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {

	pub trait CurrenciesApi<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Codec,

	{
		fn list_assets() -> Vec<CurrencyId>;

		fn free_balance(account: AccountId, asset: CurrencyId) -> Option<Balance>;

		fn total_balance(account: AccountId, asset: CurrencyId) -> Option<Balance>;
	}
}
