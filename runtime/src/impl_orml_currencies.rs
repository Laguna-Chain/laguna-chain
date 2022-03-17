use crate::{impl_orml_tokens::NativeCurrencyId, Balances, Event, Runtime, Tokens};
use orml_currencies::BasicCurrencyAdapter;
use primitives::{Amount, BlockNumber};

impl orml_currencies::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Tokens;

	// Native transfer will trigger the underlying mechanism via the underlying `Balances` module
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

	type GetNativeCurrencyId = NativeCurrencyId;
	type WeightInfo = ();
}
