#![cfg_attr(not(feature = "std"), no_std)]

use crate::{Call, Currencies, Event, Runtime};
use frame_support::parameter_types;
use primitives::{AccountId, CurrencyId, TokenId};

parameter_types! {
	pub GratitudeAccountId: AccountId = hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"].into();  // Alice, as per https://docs.substrate.io/v3/tools/subkey/#well-known-keys
	pub GratitudeCurrency: CurrencyId = CurrencyId::NativeToken(TokenId::GratitudeToken);
	pub MaxReasonLength: u32 = 128;
}

impl pallet_gratitude::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type MultiCurrency = Currencies;
	type GratitudeAccountId = GratitudeAccountId;
	type GratitudeCurrency = GratitudeCurrency;
	type MaxReasonLength = MaxReasonLength;
}
