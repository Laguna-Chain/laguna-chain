use super::*;

#[allow(unused)]
use crate::Pallet as FeeEnablement;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use primitives::{CurrencyId, TokenId};

benchmarks! {
	onboard_asset {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
		let enabled: bool = true | false;
	}: _(RawOrigin::Root, currency_id.clone(), enabled.clone())
	verify {
		assert_eq!(FeeAssets::<T>::get(currency_id), Some(enabled));
	}

	enable_asset {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id: CurrencyId = CurrencyId::Erc20([1u8;32]);
	}: _(RawOrigin::Root, currency_id.clone())
	verify {
		assert_eq!(FeeAssets::<T>::get(currency_id), Some(true));
	}

	disable_asset {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id: CurrencyId = CurrencyId::Erc20([1u8;32]);
	}: _(RawOrigin::Root, currency_id.clone())
	verify {
		assert_eq!(FeeAssets::<T>::get(currency_id), Some(false));
	}


	impl_benchmark_test_suite!(FeeEnablement, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}