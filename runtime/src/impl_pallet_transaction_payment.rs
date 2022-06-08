use crate::{FluentFee, Runtime};
use frame_support::{parameter_types, weights::IdentityFee};
use pallet_transaction_payment::CurrencyAdapter;
use primitives::Balance;

parameter_types! {
	pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	// TODO: add benchmark around cross pallet interaction between fee
	type OnChargeTransaction = FluentFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();

	type LengthToFee = IdentityFee<Balance>;
}
