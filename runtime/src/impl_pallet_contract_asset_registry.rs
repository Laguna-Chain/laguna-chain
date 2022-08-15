use crate::Runtime;
use frame_support::{parameter_types, PalletId};
use frame_system::EnsureRoot;
use primitives::AccountId;

parameter_types! {
	pub const PALLET_ID: PalletId = PalletId(*b"tkn_rgst");
	pub const MAXGAX: u64 = u64::MAX;
	pub const DEBUG: bool = true;
}

impl pallet_contract_asset_registry::Config for Runtime {
	type AllowedOrigin = EnsureRoot<AccountId>;
	type PalletId = PALLET_ID;
	type MaxGas = MAXGAX;

	type ContractDebugFlag = DEBUG;

	type WeightInfo = ();
}
