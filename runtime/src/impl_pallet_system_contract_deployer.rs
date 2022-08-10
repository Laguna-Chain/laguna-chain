use crate::{Event, Runtime};
use frame_support::{parameter_types, PalletId};

parameter_types! {
	pub const PId: PalletId = PalletId(*b"sys_depl");
}

impl pallet_system_contract_deployer::Config for Runtime {
	type Event = Event;
	type PalletId = PId;
}
