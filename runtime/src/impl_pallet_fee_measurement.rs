use frame_support::parameter_types;
use orml_traits::parameter_type_with_key;

use orml_traits::PriceProvider;
use primitives::{CurrencyId, Price};
use sp_runtime::FixedPointNumber;

use crate::Runtime;

parameter_types! {
	pub PrepaidConvertionRate: Price = Price::saturating_from_rational(11, 10);
}

pub struct DummyPriceProvider;

impl PriceProvider<CurrencyId, Price> for DummyPriceProvider {
	fn get_price(base: CurrencyId, quote: CurrencyId) -> Option<Price> {
		None
	}
}

impl pallet_fee_measurement::Config for Runtime {
	type PrepaidConversionRate = PrepaidConvertionRate;

	type AltConversionRate = DummyPriceProvider;
}
