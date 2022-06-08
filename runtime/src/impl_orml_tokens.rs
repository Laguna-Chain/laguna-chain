use crate::{
	constants::{currency::NATIVE_TOKEN, MICRO_LAGUNAS},
	Event, Runtime,
};
use frame_support::{
	parameter_types,
	traits::{ConstU32, Contains},
};
use primitives::{AccountId, Amount, Balance, CurrencyId, TokenId};

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(t: &AccountId) -> bool {
		// TODO: all account are possible to be dust-removed now
		false
	}
}

parameter_types! {
	pub const MaxLocks: u32 = 50;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {

		match currency_id {
			&CurrencyId::NativeToken(token) => {
				match token {
					TokenId::Laguna => MICRO_LAGUNAS,
					TokenId::FeeToken => MICRO_LAGUNAS,
				}
			},
			_ => Balance::max_value() // unreachable ED value for unverified currency type
		}
	};
}

type ReserveIdentifier = [u8; 8];

// parameter_types! {
// 	pub const NativeCurrencyId: CurrencyId = CurrencyId::NativeToken(NATIVE_TOKEN);
// }

// use orml's token to represent both native and other tokens
impl orml_tokens::Config for Runtime {
	type Event = Event;
	// how tokens are measured
	type Balance = Balance;
	type Amount = Amount;

	// how's tokens represented
	type CurrencyId = primitives::CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = DustRemovalWhitelist;

	type MaxReserves = ConstU32<2>;

	type ReserveIdentifier = ReserveIdentifier;
}
