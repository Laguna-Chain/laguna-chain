#![cfg(feature = "runtime-benchmarks")]

use super::*;

#[allow(unused)]
use crate::Pallet as Currencies;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use primitives::{CurrencyId, TokenId};
use sp_core::U256;

benchmarks! {
	transfer_native {
		let currency_id = CurrencyId::NativeToken(TokenId::Laguna);
		let caller: T::AccountId = whitelisted_caller();
		let to: T::AccountId = whitelisted_caller();
		let balance = 1000u128;
	}: _(RawOrigin::Signed(caller.clone()), to, currency_id, amount)
	verify {
	}



	impl_benchmark_test_suite!(Currencies, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}
