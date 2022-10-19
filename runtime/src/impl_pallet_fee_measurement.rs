use frame_support::parameter_types;

use frame_support::sp_runtime::FixedPointNumber;
use orml_traits::PriceProvider;
use primitives::{Balance, CurrencyId, Price};

use crate::{
	impl_pallet_currencies::NativeCurrencyId, impl_pallet_prepaid::PREPAIDTOKENID, Runtime,
};

parameter_types! {
	pub PrepaidConvertionRate: Price = Price::saturating_from_rational(11, 10);
}

pub struct DummyPriceProvider;

impl PriceProvider<CurrencyId, Price> for DummyPriceProvider {
	fn get_price(_base: CurrencyId, _quote: CurrencyId) -> Option<Price> {
		None
	}
}

impl pallet_fee_measurement::Config for Runtime {
	type PrepaidConversionRate = PrepaidConvertionRate;

	type AltConversionRate = DummyPriceProvider;

	type Rate = Price;

	type Balance = Balance;

	type CurrencyId = CurrencyId;

	type NativeToken = NativeCurrencyId;

	type PrepaidToken = PREPAIDTOKENID;
}
