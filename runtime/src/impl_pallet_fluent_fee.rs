use crate::{impl_pallet_currencies::NativeCurrencyId, Currencies, Event, Runtime};

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Currencies;
	type NativeCurrencyId = NativeCurrencyId;
}
