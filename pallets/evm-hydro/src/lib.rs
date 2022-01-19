#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

	use frame_support::{pallet_prelude::*, sp_runtime::app_crypto::sp_core::H160, traits::IsType};
	use frame_system::{ensure_root, pallet_prelude::OriginFor};

	use pallet_evm::pallet as pallet_evm;
	use sp_core::H256;
	use sp_std::prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_evm::Config {
		type Event: IsType<<Self as frame_system::Config>::Event> + From<Event<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T> {
		InspectOutput(Vec<u8>),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100_000)]
		pub fn inspect(
			origin: OriginFor<T>,
			contract_address: H160,
			storage_key: H256,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let value = pallet_evm::Pallet::<T>::account_storages(contract_address, storage_key);

			Self::deposit_event(Event::InspectOutput(value.as_bytes().to_vec()));

			Ok(())
		}
	}
}
