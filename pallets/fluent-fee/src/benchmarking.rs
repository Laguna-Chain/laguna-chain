use super::*;

#[allow(unused)]
use crate::Pallet as FluentFee;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use primitives::{CurrencyId, TokenId};

benchmarks! {
	where_clause {
		where
		T: crate::Config,
		T::MultiCurrency: MultiCurrency<T::AccountId, CurrencyId = CurrencyId, Balance = u128>
	}

	set_default {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);

	}: _(RawOrigin::Signed(caller.clone()), currency_id.clone())
	verify {
		assert_eq!(DefdaultFeeSource::<T>::get(caller), Some(currency_id));
	}

	unset_default {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);

		FluentFee::<T>::set_default(RawOrigin::Signed(caller.clone()).into(), currency_id.clone())?;
		assert_eq!(DefdaultFeeSource::<T>::get(caller.clone()), Some(currency_id.clone()));

	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(DefdaultFeeSource::<T>::get(caller), None);
	}

	impl_benchmark_test_suite!(FluentFee, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}
