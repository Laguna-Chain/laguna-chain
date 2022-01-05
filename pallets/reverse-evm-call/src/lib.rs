#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;

pub use pallet::*;

#[frame_support::pallet]
mod pallet {
    use sp_core::H160;

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_evm::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        #[pallet::constant]
        type Caller: Get<H160>;

        #[pallet::constant]
        type TargetAddress: Get<(H160, Vec<u8>)>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        TargetExecuted,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(100_000)]
        pub fn delegate_call(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let (contract, call) = <T::TargetAddress>::get();

            let res = pallet_evm::Pallet::<T>::call(
                origin,
                <T::Caller>::get(),
                contract,
                call,
                0.into(),
                1000_000,
                0.into(),
                None,
                None,
                vec![],
            )?;

            Self::deposit_event(Event::TargetExecuted);

            Ok(res)
        }
    }
}
