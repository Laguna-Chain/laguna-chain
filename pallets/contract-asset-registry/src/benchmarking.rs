use super::*;
use crate::Pallet as ContractAssetRegistry;
#[allow(unused)]
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	register_asset {
		let caller: T::AccountId = whitelisted_caller();
		// Any arbitrary AccountId
		let contract_address: T::AccountId = account("contract_address", 0, 0);

	} : _(RawOrigin::Root, contract_address.clone(), true)
		verify {
			assert_eq!(RegisteredAsset::<T>::get(contract_address), Some(true));
	}

	suspend_asset {
		// Any arbitrary AccountId
		let contract_address: T::AccountId = account("contract_address", 0, 0);
		// First enable the asset
		ContractAssetRegistry::<T>::register_asset(RawOrigin::Root.into(), contract_address.clone(), true)?;

	} : _(RawOrigin::Root, contract_address.clone())
		verify {
			assert_eq!(RegisteredAsset::<T>::get(contract_address), Some(false));
	}

	unregister_asset {
		// Any arbitrary AccountId
		let contract_address: T::AccountId = account("contract_address", 0, 0);
		// First enable the asset
		ContractAssetRegistry::<T>::register_asset(RawOrigin::Root.into(), contract_address.clone(), true)?;

	} : _(RawOrigin::Root, contract_address.clone())
		verify {
			assert_eq!(RegisteredAsset::<T>::get(contract_address), None);
	}

	impl_benchmark_test_suite!(ContractAssetRegistry, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}
