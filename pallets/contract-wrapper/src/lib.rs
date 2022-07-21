//! ## pallet-contract-wrapper
//!
//! This pallet allows system contracts to be deployed at fixed addresses

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::HasCompact;
	use frame_support::{pallet_prelude::*, traits::Currency};
	use frame_system::{pallet_prelude::*, RawOrigin};
	use pallet_contracts::weights::WeightInfo;
	use sp_core::crypto::UncheckedFrom;
	use sp_runtime::AccountId32;
	use sp_std::{fmt::Debug, vec::Vec};

	type CodeHash<T> = <T as frame_system::Config>::Hash;
	type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		#[pallet::weight(
			T::WeightInfo::instantiate_with_code(code.len() as u32, salt.len() as u32)
			.saturating_add(*gas_limit)
		)]
		pub fn instantiate_with_code(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
			#[pallet::compact] gas_limit: Weight,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
			code: Vec<u8>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let zero_addr = AccountId32::new([0u8; 32]);
			let zero_addr = T::AccountId::decode(&mut zero_addr.as_ref()).unwrap();
			let zero_addr = RawOrigin::Signed(zero_addr);

			pallet_contracts::Pallet::<T>::instantiate_with_code(
				zero_addr.into(),
				value,
				gas_limit,
				storage_deposit_limit,
				code,
				data,
				salt,
			)
		}

		#[pallet::weight(
			T::WeightInfo::instantiate(salt.len() as u32).saturating_add(*gas_limit)
		)]
		pub fn instantiate(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
			#[pallet::compact] gas_limit: Weight,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
			code_hash: CodeHash<T>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let zero_addr = AccountId32::new([0u8; 32]);
			let zero_addr = T::AccountId::decode(&mut zero_addr.as_ref()).unwrap();
			let zero_addr = RawOrigin::Signed(zero_addr);

			pallet_contracts::Pallet::<T>::instantiate(
				zero_addr.into(),
				value,
				gas_limit,
				storage_deposit_limit,
				code_hash,
				data,
				salt,
			)
		}

		#[pallet::weight(T::WeightInfo::upload_code(code.len() as u32))]
		pub fn upload_code(
			origin: OriginFor<T>,
			code: Vec<u8>,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let zero_addr = AccountId32::new([0u8; 32]);
			let zero_addr = T::AccountId::decode(&mut zero_addr.as_ref()).unwrap();
			let zero_addr = RawOrigin::Signed(zero_addr);

			pallet_contracts::Pallet::<T>::upload_code(
				zero_addr.into(),
				code,
				storage_deposit_limit,
			)
		}

		#[pallet::weight(T::WeightInfo::remove_code())]
		pub fn remove_code(
			origin: OriginFor<T>,
			code_hash: CodeHash<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let zero_addr = AccountId32::new([0u8; 32]);
			let zero_addr = T::AccountId::decode(&mut zero_addr.as_ref()).unwrap();
			let zero_addr = RawOrigin::Signed(zero_addr);

			pallet_contracts::Pallet::<T>::remove_code(zero_addr.into(), code_hash)
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub addr: Vec<[u8; 32]>,
		pub code: Vec<Vec<u8>>,
		pub data: Vec<Vec<u8>>,
		pub gas_limit: Weight,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				addr: Default::default(),
				code: Default::default(),
				data: Default::default(),
				gas_limit: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn build(&self) {
			assert_eq!(self.addr.len(), self.code.len());
			let sz = self.addr.len();

			let zero_addr = AccountId32::new([0u8; 32]);
			let zero_addr = T::AccountId::decode(&mut zero_addr.as_ref()).unwrap();

			for i in 0..sz {
				pallet_contracts::Pallet::<T>::bare_upload_code(
					zero_addr.clone(),
					self.code[i].clone(),
					None,
				)
				.expect("Code not uploaded");
			}
		}
	}
}
