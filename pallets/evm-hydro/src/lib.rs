#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use frame_support::pallet_prelude::*;
    use frame_system::{ensure_root, pallet_prelude::OriginFor};

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_evm::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(100_000_000)]
        pub fn trigger(origin: OriginFor<T>, code: Vec<u8>) -> DispatchResult {
            let _ = ensure_root(origin);

            Ok(())
        }
    }
}
