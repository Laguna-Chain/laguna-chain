//! ## pallet-system-contract-deployer
//!
//! This pallet allows system contracts to be deployed at fixed addresses.
//! It is tightly-coupled with the pallet-contract and exposes privileged extrinsics

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
	use frame_support::{pallet_prelude::*, sp_runtime, sp_std, traits::Currency, PalletId};
	use frame_system::{pallet_prelude::*, RawOrigin};
	use pallet_contracts::weights::WeightInfo;
	use sp_core::crypto::UncheckedFrom;
	use sp_runtime::{
		traits::{AccountIdConversion, Hash},
		AccountId32,
	};
	use sp_std::{fmt::Debug, vec::Vec};
	type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

	type CodeHash<T> = <T as frame_system::Config>::Hash;
	type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn is_system_contract)]
	pub type SystemContracts<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	// Stores the next possible sequential address value in integer form
	#[pallet::storage]
	pub type NextAddress<T: Config> = StorageValue<_, u32>;

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
		/// Instantiates a new system-contract from the supplied `code` optionally transferring
		/// some balance and optionally providing the destined address.
		///
		/// Setting destined_address to None evaluates the next sequential address (starts from
		/// 0x01)
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
			destined_address: Option<[u8; 32]>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let destined_address =
				destined_address.unwrap_or_else(|| Self::get_next_available_bytes());

			let deployer: AccountIdOf<T> =
				T::PalletId::get().try_into_account().expect("Invalid PalletId");

			let output = pallet_contracts::Pallet::<T>::instantiate_with_code(
				RawOrigin::Signed(deployer.clone()).into(),
				value,
				gas_limit,
				storage_deposit_limit,
				code.clone(),
				data,
				destined_address.to_vec(),
			);

			let final_addr = pallet_contracts::Pallet::<T>::contract_address(
				&deployer,
				&<<T as frame_system::Config>::Hashing>::hash(&code[..]),
				&destined_address,
			);

			// @dev: coupling or event extraction?
			if output.is_ok() {
				SystemContracts::<T>::insert(final_addr.clone(), true);
				Self::deposit_event(Event::<T>::Created(final_addr));
			}

			output
		}

		/// Instantiates a contract from a previously deployed wasm binary.
		///
		/// Setting destined_address to None evaluates the next sequential address (starts from
		/// 0x01)
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
			destined_address: Option<[u8; 32]>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let destined_address =
				destined_address.unwrap_or_else(|| Self::get_next_available_bytes());

			let output = pallet_contracts::Pallet::<T>::instantiate(
				RawOrigin::Signed(T::PalletId::get().try_into_account().expect("Invalid PalletId"))
					.into(),
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

		/// Upload new `code` without instantiating a contract from it.
		#[pallet::weight(T::WeightInfo::upload_code(code.len() as u32))]
		pub fn upload_code(
			origin: OriginFor<T>,
			code: Vec<u8>,
			storage_deposit_limit: Option<<BalanceOf<T> as codec::HasCompact>::Type>,
		) -> DispatchResult {
			ensure_root(origin)?;

			pallet_contracts::Pallet::<T>::upload_code(
				RawOrigin::Signed(T::PalletId::get().try_into_account().expect("Invalid PalletId"))
					.into(),
				code,
				storage_deposit_limit,
			)
		}

		/// Remove the code stored under `code_hash` and refund the deposit to its owner.
		///
		/// A code can only be removed by its original uploader (its owner) and only if it is
		/// not used by any contract.
		#[pallet::weight(T::WeightInfo::remove_code())]
		pub fn remove_code(
			origin: OriginFor<T>,
			code_hash: CodeHash<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			pallet_contracts::Pallet::<T>::remove_code(
				RawOrigin::Signed(T::PalletId::get().try_into_account().expect("Invalid PalletId"))
					.into(),
				code_hash,
			)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns a list of system-contracts deployed on-chain
		pub fn get_all_system_contracts() -> Vec<T::AccountId> {
			SystemContracts::<T>::iter_keys().collect()
		}

		// Helper function used to find the bytes of the next available sequential address
		fn get_next_available_bytes() -> [u8; 32] {
			let mut counter = NextAddress::<T>::get().unwrap_or(1);
			loop {
				let hex = scale_info::prelude::format!("{:064x}", counter);
				let mut byte = [0u8; 32];
				hex::decode_to_slice(hex, &mut byte).unwrap();
				let addr = AccountId32::from(byte);
				let addr = T::AccountId::decode(&mut addr.as_ref()).unwrap();
				if !Self::is_system_contract(addr.clone()) {
					return byte
				}
				counter += 1;
			}
		}

		/// Returns the next available sequential address where the contract can be deployed
		pub fn get_next_available_address() -> T::AccountId {
			let byte = Self::get_next_available_bytes();
			let addr = AccountId32::from(byte);
			T::AccountId::decode(&mut addr.as_ref()).unwrap()
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

			// The first available destined address is set to 0x01
			NextAddress::<T>::put(1);

			for i in 0..sz {
				pallet_contracts::Pallet::<T>::bare_upload_code(
					T::PalletId::get().try_into_account().expect("Invalid PalletId"),
					self.code[i].clone(),
					None,
				)
				.expect("Code not uploaded");
			}
		}
	}
}
