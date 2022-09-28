use primitives::AccountId;
use sp_core::H160;
use sp_runtime::traits::StaticLookup;

use crate::Runtime;

impl pallet_evm_compat::Config for Runtime {
	type AddrLookup = AccountLookup;
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
