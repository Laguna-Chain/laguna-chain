#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub mod benchmarking;

pub(crate) mod weights;
pub use crate::weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo; // allow benchmarking mode to customized weight calculation
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Report,
	}

	#[pallet::storage]
	pub type Counter<T> = StorageValue<_, u32>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// subsitude WeightInfo by benchmarking or manually

		#[pallet::weight(T::WeightInfo::dummy())]
		pub fn dummy(_: OriginFor<T>) -> DispatchResult {
			log::info!(target: "pallet_rando", "successfull");
			Self::deposit_event(Event::Report);

			Counter::<T>::try_mutate(|counts| {
				match counts {
					Some(val) => {
						*val += 1;
					},
					None => {
						*counts = Some(1);
					},
				};
				let out: DispatchResult = Ok(());
				out
			})?;

			Ok(())
		}
	}
}
