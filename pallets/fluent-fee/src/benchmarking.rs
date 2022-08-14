use super::*;

#[allow(unused)]
use crate::Pallet as FluentFee;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use primitives::{CurrencyId, TokenId};

benchmarks! {
	set_default {
		let s = CurrencyId::NativeToken(TokenId::Laguna);
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller.clone()), s)
	verify {
		assert_eq!(DefdaultFeeSource::<T>::get(caller), Some(s));
	}

	unset_default {
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(DefdaultFeeSource::<T>::get(caller), None);
	}


	impl_benchmark_test_suite!(FluentFee, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}
