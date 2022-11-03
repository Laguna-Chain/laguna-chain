use frame_support::parameter_types;

use frame_support::sp_runtime::traits::ConvertInto;
use primitives::CurrencyId;

use crate::{constants::LAGUNA_NATIVE_CURRENCY, ContractAssetsRegistry, Runtime, Tokens};

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = LAGUNA_NATIVE_CURRENCY;
}

impl pallet_currencies::Config for Runtime {
	type NativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type ContractAssets = ContractAssetsRegistry;
	type ConvertIntoAccountId = ConvertInto;
}
