use crate::{impl_orml_tokens::NativeCurrencyId, Contracts, Runtime};
use frame_support::{parameter_types, PalletId};
use orml_currencies::BasicCurrencyAdapter;
use orml_tokens::CurrencyAdapter;

parameter_types! {
	pub const PALLET_ID: PalletId = PalletId(*b"tkn_rgst");
	pub const MAXGAX: u64 = u64::MAX;
	pub const DEBUG: bool = true;
}

impl pallet_contract_asset_registry::Config for Runtime {
	type PalletId = PALLET_ID;
	type MaxGas = MAXGAX;

	type Currency = CurrencyAdapter<Runtime, NativeCurrencyId>;
	type ContractsPlatform = Contracts;
	type ContractDebugFlag = DEBUG;
}
