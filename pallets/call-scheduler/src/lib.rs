//# # Call Scheduler
//#
//# this module provides call scheduling functionalities

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, dispatch::{Dispatchable, DispatchError, DispatchResult, Parameter}, traits::{schedule::{self, DispatchTime}, Get, IsType}, weights::{GetDispatchInfo, Weight}};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::TargetedFeeAdjustment;
use primitives::{CurrencyId, TokenId};

pub use pallet::*;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccoundIdOf<T>>>::Balance;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Call: Dispatchable + Parameter + GetDispatchInfo;
		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		#[pallet::constant]
		type MaxCallsPerBlock: Get<u32>;

		#[pallet::constant]
		type ScheduleReserveAccountId: Get<Self::AccountId>;

		#[pallet::constant]
		type FundsLockerAccountId: Get<Self::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// call scheduled
		Scheduled {},
	}

	#[pallet::error]
	pub enum Error<T> {
		InsufficientScheduleBalance,
	}

    // The average targeted fee adjustment computed across blocks based on the network congestion
    #[pallet::storage]
    pub type AvgTargetedFeeAdjustment<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;



    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // execute the scheduled tasks
        fn on_initialize(now: T::BlockNumber) -> Weight {

        }

        // perform bookkeeping 
        fn on_finalize(now: T::BlockNumber) {

        }
    }
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1000_000)]
        pub fn schedule_call(origin: OriginFor<T>, call: Box<T::Call>, maybe_periodic: )
    }
}
