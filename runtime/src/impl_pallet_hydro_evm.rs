use crate::{Event, Runtime};

impl evm_hydro::Config for Runtime {
	type Event = Event;
}
