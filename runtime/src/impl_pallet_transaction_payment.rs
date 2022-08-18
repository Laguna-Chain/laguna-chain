use crate::{Event, FluentFee, Runtime};
use frame_support::{parameter_types, weights::IdentityFee};
use primitives::Balance;

parameter_types! {
	pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	// TODO: add benchmark around cross pallet interaction between fee
	type Event = Event;
	type OnChargeTransaction = FluentFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();

	type LengthToFee = IdentityFee<Balance>;
}
