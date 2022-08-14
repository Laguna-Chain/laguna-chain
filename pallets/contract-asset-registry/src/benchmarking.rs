use super::*;

#[allow(unused)]
use crate::Pallet as ContractAssetRegistry;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use primitives::{CurrencyId, TokenId};

benchmarks! {
	register_asset {
		let asset_contract_address: T::AccountId = whitelisted_caller();
		let caller: T::AccountId = whitelisted_caller();
		let enabled = true;
	}: _(RawOrigin::Root, asset_contract_address.clone(), enabled.clone())
	verify {
		assert_eq!(RegisteredAsset::<T>::get(asset_contract_address), Some(enabled));
	}

	suspend_asset {
		let caller: T::AccountId = whitelisted_caller();
		let asset_contract_address: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, asset_contract_address.clone())
	verify {
		assert_eq!(RegisteredAsset::<T>::get(asset_contract_address), Some(false));
	}

	unregister_asset {
		let caller: T::AccountId = whitelisted_caller();
		let asset_contract_address: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, asset_contract_address.clone())
	verify {
		assert_eq!(RegisteredAsset::<T>::get(asset_contract_address), None);
	}


	impl_benchmark_test_suite!(ContractAssetRegistry, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}
