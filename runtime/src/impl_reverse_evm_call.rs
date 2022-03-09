#![cfg_attr(not(feature = "std"), no_std)]

use crate::{Event, Runtime};
use frame_support::parameter_types;
use sp_core::H160;
use sp_std::prelude::*;

parameter_types! {
	pub Caller: H160 = H160::from_slice(&hex_literal::hex!("37C54011486B797FAA83c5CF6de88C567843a23F"));
	pub TargetAddress: (H160, Vec<u8>) = (H160::from_low_u64_be(9001), precompile_utils::EvmDataWriter::new_with_selector(pallet_rando_precompile::Action::CallRando).build());
}

impl pallet_reverse_evm_call::Config for Runtime {
	type Event = Event;

	type Caller = Caller;

	type TargetAddress = TargetAddress;
}
