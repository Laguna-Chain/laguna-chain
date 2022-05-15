use frame_support::traits::Currency;
use orml_tokens::CurrencyAdapter;
use primitives::{AccountId, Amount, BlockNumber};
use sp_core::sr25519;
use sp_runtime::traits::Convert;

use crate::{
	impl_orml_tokens::NativeCurrencyId, Balances, ContractAssetsRegistry, Runtime, Tokens,
};

impl pallet_currencies::Config for Runtime {
	type NativeCurrency = CurrencyAdapter<Runtime, NativeCurrencyId>;
	type NativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type ContractAssets = ContractAssetsRegistry;
	type ConvertIntoAccountId = AddressConvert;
}

struct AddressConvert;

impl Convert<sr25519::Public, AccountId> for AddressConvert {
	fn convert(a: sr25519::Public) -> AccountId {
		a.into()
	}
}
