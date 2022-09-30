use primitives::{AccountId, Balance};
use sp_core::{H160, U256};
use sp_runtime::traits::{Convert, StaticLookup};

use crate::Runtime;

impl pallet_evm_compat::Config for Runtime {
	type AddrLookup = AccountLookup;
	type BalanceConvert = BalanceConvert;
}

pub struct BalanceConvert;

impl Convert<U256, Balance> for BalanceConvert {
	fn convert(a: U256) -> Balance {
		a.as_u128()
	}
}

pub struct AccountLookup;

impl StaticLookup for AccountLookup {
	type Source = H160;

	type Target = AccountId;

	fn lookup(s: Self::Source) -> Result<Self::Target, frame_support::error::LookupError> {
		todo!()
	}

	fn unlookup(t: Self::Target) -> Self::Source {
		todo!()
	}
}
