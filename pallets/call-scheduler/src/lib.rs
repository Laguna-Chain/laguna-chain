//# # Call Scheduler
//#
//# this module provides call scheduling functionalities

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use frame_support::{
	dispatch::{DispatchError, DispatchResult, Dispatchable, Parameter},
	pallet_prelude::*,
	sp_runtime::FixedPointNumber,
	traits::{
		schedule::{self, DispatchTime},
		EstimateCallFee, Get, IsType, StorageVersion,
	},
	weights::{GetDispatchInfo, Weight},
};
use frame_system::pallet_prelude::*;
use orml_traits::{arithmetic::One, MultiCurrency};
use pallet_transaction_payment::NextFeeMultiplier;
use primitives::{CurrencyId, TokenId};
use scale_info::TypeInfo;
use sp_std::marker::PhantomData;

pub use pallet::*;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
// information regarding a call to be scheduled in future
#[cfg_attr(any(feature = "std", test), derive(PartialEq, Eq))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Scheduled<Call, BlockNumber, PalletsOrigin, AccountId> {
	maybe_id: Option<Vec<u8>>,
	priority: schedule::Priority,
	call: Call,
	maybe_periodic: Option<schedule::Period<BlockNumber>>,
	origin: PalletsOrigin,
	_phantom: PhantomData<AccountId>,
}

pub type ScheduleInfo<T> = Scheduled<
	<T as Config>::Call,
	<T as frame_system::Config>::BlockNumber,
	<T as Config>::PalletsOrigin,
	<T as frame_system::Config>::AccountId,
>;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Call: Dispatchable + Parameter + GetDispatchInfo;
		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>
			+ Codec
			+ Clone
			+ Eq
			+ TypeInfo;

		#[pallet::constant]
		type MaxCallsPerBlock: Get<u32>;

		// The account that pays for the scheduled calls, this balance can be topped up from the
		// locked funds from ScheduleReserve
		#[pallet::constant]
		type SchedulePrepayAccountId: Get<Self::AccountId>;

		// The Account where users lock funds for prepaying their scheduled calls
		#[pallet::constant]
		type ScheduleReserveAccountId: Get<Self::AccountId>;
	}

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	// #[pallet::generate_store(pub(super) trait Store)]
	// #[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
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
	#[pallet::getter(fn avg_next_fee_multiplier)]
	pub type AvgNextFeeMultiplier<T: Config> = StorageValue<_, u128, ValueQuery>;

	// Scheduled calls to be executed, indexed by block number that they should be executed on.
	#[pallet::storage]
	pub type CallsScheduled<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<Option<ScheduleInfo<T>>>, ValueQuery>;

	// Scheduled calls halted due to insufficient funds, indexed by the dispatch owner
	#[pallet::storage]
	pub type HaltedQueue<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, ScheduleInfo<T>, OptionQuery>;

	// Tracks the funds locked in by users to prepay/top-up their scheduled calls
	#[pallet::storage]
	pub type ScheduleReserveBalances<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	// Keeps track of the user balance for the scheduled calls
	#[pallet::storage]
	pub type SchedulePrepayBalances<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// execute the scheduled tasks
		// fn on_initialize(now: T::BlockNumber) -> Weight {
		// 	// schedule logic
		// 	0u64
		// }

		// perform bookkeeping
		fn on_finalize(now: T::BlockNumber) {
			let current_fee_multipler = NextFeeMultiplier::<T>::get().into_inner();
			let running_avg: u128 = unsafe {
				current_fee_multipler.saturating_div(now as u128).saturating_add(
					Self::avg_next_fee_multiplier()
						.saturating_mul((now as u128 - 1u128).saturating_div(now as u128)),
				)};
			AvgNextFeeMultiplier::<T>::put(running_avg);
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000_000)]
		pub fn schedule_call(
			origin: OriginFor<T>,
			when: T::BlockNumber,
			call: <T as pallet::Config>::Call,
			maybe_periodic: Option<schedule::Period<T::BlockNumber>>,
			priority: schedule::Priority,
		) -> DispatchResult {
			let info = call.get_dispatch_info();
			// Base Fee Calculation: find capped base extrinsic weight , then compute weight_to_fee.
			// let base_weight: Weight = (T::BlockWeights::get().get(info.class).base_extrinsic)
			// 	.min(T::BlockWeights::get().max_block);
			// let base_fee = T::TransactionPayment::WeightToFee::calc(&base_weight);
			// // Compute the len fee
			// let len_fee =
			// 	T::TransactionPayment::LengthToFee::calc(&(call.encoded_size() as u32 as Weight));
			// // Get the average next multiplier fee
			// let avg_next_multiplier_fee = Self::AvgTargetedFeeAdjustment::get();
			// // Compute the weight fee
			// let weight_fee = T::TransactionPayment::WeightToFee::calc(
			// 	&info.weight.min(T::BlockWeights::get().max_block),
			// );
			// let total_fee = base_fee + len_fee + (avg_next_multiplier_fee * weight_fee);
			Ok(())
		}
	}
}
