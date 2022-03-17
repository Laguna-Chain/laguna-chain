use crate::{impl_orml_tokens::NativeCurrencyId, Currencies, Event, Runtime};

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Currencies;
	type NativeCurrencyId = NativeCurrencyId;
}
