#![cfg_attr(not(feature = "std"), no_std)]

use crate::{Call, Event, Runtime};

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}
