use crate::{Event, Runtime};

impl pallet_rando::Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
}
