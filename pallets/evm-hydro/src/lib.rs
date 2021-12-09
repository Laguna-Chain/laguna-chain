#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use frame_support::{pallet_prelude::*, traits::IsType};
    use frame_system::{ensure_root, pallet_prelude::OriginFor};
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Wrapper: Wrapper;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ExaminePass,
        ExamineFail,
    }

    impl<T: Config> From<bool> for Event<T> {
        fn from(val: bool) -> Self {
            if val {
                Event::ExaminePass
            } else {
                Event::ExamineFail
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(100_000_000)]
        pub fn trigger(origin: OriginFor<T>, code: Vec<u8>) -> DispatchResult {
            let _ = ensure_root(origin)?;

            Self::deposit_event(T::Wrapper::examine(code).into());

            Ok(())
        }
    }

    // allow other trait to specify their pallet behaviour
    pub trait Wrapper {
        fn examine(code: Vec<u8>) -> bool;
    }
}
