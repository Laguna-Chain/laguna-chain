//! ## pallet-system-contract-deployer
//!
//! This pallet allows system contracts to be deployed at fixed addresses

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod fixed_address;
pub use fixed_address::CustomAddressGenerator;

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
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn deploying_key)]
	pub type DeployingKey<T: Config> = StorageValue<_, <T as frame_system::Config>::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn is_system_contract)]
	pub type SystemContracts<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [contract_address]
		Created(T::AccountId),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		#[pallet::weight(
			T::WeightInfo::instantiate_with_code(code.len() as u32, 32_u32)
			.saturating_add(*gas_limit)
		)]
		pub fn instantiate_with_code(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
			#[pallet::compact] gas_limit: Weight,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
			code: Vec<u8>,
			data: Vec<u8>,
			destined_address: [u8; 32],
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let output = pallet_contracts::Pallet::<T>::instantiate_with_code(
				RawOrigin::Signed(Self::deploy_key()).into(),
				value,
				gas_limit,
				storage_deposit_limit,
				code,
				data,
				destined_address.to_vec(),
			);

			// @dev: coupling or event extraction?
			if output.is_ok() {
				let contract_addr = AccountId32::from(destined_address);
				let contract_addr = T::AccountId::decode(&mut contract_addr.as_ref())
					.expect("Cannot create an AccountId from the given salt");

				SystemContracts::<T>::insert(contract_addr.clone(), true);
				Self::deposit_event(Event::<T>::Created(contract_addr));
			}

			output
		}

		#[pallet::weight(
			T::WeightInfo::instantiate(32_u32).saturating_add(*gas_limit)
		)]
		pub fn instantiate(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
			#[pallet::compact] gas_limit: Weight,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
			code_hash: CodeHash<T>,
			data: Vec<u8>,
			destined_address: [u8; 32],
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let output = pallet_contracts::Pallet::<T>::instantiate(
				RawOrigin::Signed(Self::deploy_key()).into(),
				value,
				gas_limit,
				storage_deposit_limit,
				code_hash,
				data,
				destined_address.to_vec(),
			);

			// @dev: coupling or event extraction?
			if output.is_ok() {
				let contract_addr = AccountId32::from(destined_address);
				let contract_addr = T::AccountId::decode(&mut contract_addr.as_ref())
					.expect("Cannot create an AccountId from the given salt");

				SystemContracts::<T>::insert(contract_addr.clone(), true);
				Self::deposit_event(Event::<T>::Created(contract_addr));
			}

			output
		}

		#[pallet::weight(T::WeightInfo::upload_code(code.len() as u32))]
		pub fn upload_code(
			origin: OriginFor<T>,
			code: Vec<u8>,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
		) -> DispatchResult {
			ensure_root(origin)?;

			pallet_contracts::Pallet::<T>::upload_code(
				RawOrigin::Signed(Self::deploy_key()).into(),
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

			pallet_contracts::Pallet::<T>::remove_code(
				RawOrigin::Signed(Self::deploy_key()).into(),
				code_hash,
			)
		}
	}

	impl<T: Config> Pallet<T> {
		fn deploy_key() -> T::AccountId {
			Self::deploying_key().expect("Key is set at genesis")
		}

		pub fn get_all_system_contracts() -> Vec<T::AccountId> {
			SystemContracts::<T>::iter_keys().collect()
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub deploying_key: T::AccountId,
		pub addr: Vec<[u8; 32]>,
		pub code: Vec<Vec<u8>>,
		pub data: Vec<Vec<u8>>,
		pub gas_limit: Weight,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			let zero_addr = AccountId32::new([0u8; 32]);
			let zero_addr = T::AccountId::decode(&mut zero_addr.as_ref()).unwrap();
			Self {
				deploying_key: zero_addr,
				addr: Default::default(),
				code: Default::default(),
				data: Default::default(),
				gas_limit: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn build(&self) {
			assert_eq!(self.addr.len(), self.code.len());
			let sz = self.addr.len();

			DeployingKey::<T>::put(self.deploying_key.clone());

			for i in 0..sz {
				pallet_contracts::Pallet::<T>::bare_upload_code(
					crate::Pallet::<T>::deploying_key().unwrap(),
					self.code[i].clone(),
					None,
				)
				.expect("Code not uploaded");
			}
		}
	}
}
