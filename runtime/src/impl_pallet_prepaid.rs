use frame_support::{parameter_types, traits::Get, PalletId};
use primitives::{CurrencyId, TokenId};
use sp_runtime::{FixedPointNumber, FixedU128};

use crate::{impl_pallet_currencies::NativeCurrencyId, Currencies, Event, Runtime};

parameter_types! {

	pub const PALLETID: PalletId = PalletId(*b"pretoken");
	pub const PREPAIDTOKENID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);
}

pub struct MaxRatio;

impl Get<FixedU128> for MaxRatio {
	fn get() -> FixedU128 {
		FixedU128::saturating_from_rational(20_u128, 100_u128)
	}
}

impl pallet_prepaid::Config for Runtime {
	type Event = Event;

	type MaxPrepaidRaio = MaxRatio;

	type MultiCurrency = Currencies;

	type NativeCurrencyId = NativeCurrencyId;

	type PrepaidCurrencyId = PREPAIDTOKENID;

	type PalletId = PALLETID;
}
