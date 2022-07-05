//# # Call Scheduler
//#
//# this module provides call scheduling functionalities

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, dispatch::{Dispatchable, DispatchError, DispatchResult, Parameter}, traits::{schedule::{self, DispatchTime}, Get, IsType}, weights::{GetDispatchInfo, Weight}};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::{OnChargeTransaction, MultiplierUpdate};
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
        // The transaction_payment pallet
        type TransactionPayment: OnChargeTransaction + MultiplierUpdate;

		#[pallet::constant]
		type MaxCallsPerBlock: Get<u32>;

        // The account that pays for the scheduled calls, this balance can be topped up from the locked funds from ScheduleReserve 
		#[pallet::constant]
		type SchedulePrepayAccountId: Get<Self::AccountId>;

        // The Account where users lock funds for prepaying their scheduled calls
		#[pallet::constant]
		type ScheduleReserveAccountId: Get<Self::AccountId>;
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

    // The average base extrinsic fee computed across blocks
    #[pallet::storage]
    pub type AvgBaseFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    // Scheduled calls to be executed, indexed by block number that they should be executed on.
    #[pallet::storage]
    pub type CallsScheduled<T: Config> = StorageMap<_, Twox64Concat, T::BlockNumber, Vec<Option<ScheduleInfo<T>>>, ValueQuery>;

    // Scheduled calls halted due to insufficient funds, indexed by the dispatch owner
    #[pallet::storage]
    pub type HaltedQueue<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ScheduleInfo<T>, ValueQuery>;

    // Tracks the funds locked in by users to prepay/top-up their scheduled calls
    #[pallet::storage]
    pub type ScheduleReserveBalances<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    // Keeps track of the user balance for the scheduled calls
    #[pallet::storage]
    pub type SchedulePrepayBalances<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;



    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // execute the scheduled tasks
        fn on_initialize(now: T::BlockNumber) -> Weight {
        }

        // perform bookkeeping 
        fn on_finalize(now: T::BlockNumber) {
            let current_fee_multipler = T::TransactionPayment::next_fee_multiplier();
            let running_avg = BalanceOf<T>::from(NextFeeMultiplier::get().into_inner()) / n + AvgTargetedFeeAdjustment::get() * BalanceOf<T>::from((now - 1) / now);
            let base_fee_running = 
        }
    }
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1000_000)]
        pub fn schedule_call(origin: OriginFor<T>, call: Box<T::Call>, maybe_periodic: T) -> DispatchResult {
            let info = call.get_dispatch_info();
            // Base Fee Calculation: find capped base extrinsic weight , then compute weight_to_fee. 
            let base_weight: Weight = (T::BlockWeights::get().get(info.class).base_extrinsic).min(T::BlockWeights::get().max_block);
            let base_fee = T::TransactionPayment::WeightToFee::calc(&base_weight);
            // Compute the len fee
            let len_fee = T::TransactionPayment::LengthToFee::calc(&(call.encoded_size() as u32 as Weight));
            // Get the average next multiplier fee
            let avg_next_multiplier_fee = Self::AvgTargetedFeeAdjustment::get();
            // Compute the weight fee
            let weight_fee = T::TransactionPayment::WeightToFee::calc(&info.weight.min(T::BlockWeights::get().max_block));
            let total_fee = base_fee + len_fee + (avg_next_multiplier_fee * weight_fee);
        }
    }
}
