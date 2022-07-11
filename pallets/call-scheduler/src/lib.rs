//# # Call Scheduler
//#
//# this module provides call scheduling functionalities

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode, EncodeLike};
use frame_support::{
	dispatch::{DispatchError, DispatchResult, Dispatchable, Parameter},
	pallet_prelude::*,
	sp_runtime::traits::{Hash, One, Saturating, Zero},
	traits::{
		schedule::{self, DispatchTime},
		EnsureOrigin, Get, IsType, OriginTrait, StorageVersion,
	},
	weights::{GetDispatchInfo, PostDispatchInfo, Weight},
};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::NextFeeMultiplier;
use primitives::CurrencyId;
use scale_info::TypeInfo;
use sp_std::marker::PhantomData;

pub use pallet::*;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
/// Just a simple index for naming period tasks.
pub type PeriodicIndex = u32;
/// The location of a scheduled task that can be used to remove it.
pub type TaskAddress<BlockNumber> = (BlockNumber, u32);
/// Type representing the Scheduled struct's hash that is used an identity for a scheduled call
pub type ScheduleHash<T> = <T as frame_system::Config>::Hash;

// information regarding a call to be scheduled in future
#[cfg_attr(any(feature = "std", test), derive(PartialEq, Eq))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Scheduled<Hash, Call, BlockNumber, PalletsOrigin, AccountId> {
	id: Hash,
	priority: schedule::Priority,
	call: Call,
	maybe_periodic: Option<schedule::Period<BlockNumber>>,
	origin: PalletsOrigin,
	_phantom: PhantomData<AccountId>,
}

pub type ScheduleInfo<T> = Scheduled<
	ScheduleHash<T>,
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
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Call: Parameter
			+ Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>;

		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>
			+ Codec
			+ Clone
			+ Eq
			+ TypeInfo;

		/// The aggregated origin which the dispatch will take.
		type Origin: OriginTrait<PalletsOrigin = Self::PalletsOrigin>
			+ From<Self::PalletsOrigin>
			+ IsType<<Self as frame_system::Config>::Origin>;

		/// Required origin to schedule or cancel calls.
		type ScheduleOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		#[pallet::constant]
		type NativeAssetId: Get<CurrencyId>;

		#[pallet::constant]
		type MaxScheduledPerBlock: Get<u32>;

		#[pallet::constant]
		type MaximumWeight: Get<Weight>;

		// The account that pays for the scheduled calls, this balance can be topped up from the
		// locked funds from ScheduleReserve
		#[pallet::constant]
		type SchedulePrepayAccountId: Get<Self::AccountId>;

		// The Account where users lock funds for prepaying their scheduled calls
		#[pallet::constant]
		type ScheduleLockedFundAccountId: Get<Self::AccountId>;
	}

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// call scheduled
		Scheduled {
			when: T::BlockNumber,
			index: u32,
		},
		/// Dispatched some task.
		Dispatched {
			task: TaskAddress<T::BlockNumber>,
			id: ScheduleHash<T>,
			result: DispatchResult,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		///
		InsufficientBalanceToResumeCall,
		TargetBlockNumberInPast,
	}

	/// Keep track of the halted calls (its id or hash) that have not been topped up within the
	/// expiry time. If the origin recharges the schedule call, it will be removed from halted queue
	/// and placed back in the ScheduleCallQueue, otherwise it will be removed permanently.
	#[pallet::storage]
	pub type DeathBowl<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<ScheduleHash<T>>, ValueQuery>;
	/// Lookup from identity to the block number and index of the task.
	#[pallet::storage]
	pub(crate) type Lookup<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, TaskAddress<T::BlockNumber>>;

	// The average targeted fee adjustment computed across blocks based on the network congestion
	#[pallet::storage]
	#[pallet::getter(fn avg_next_fee_multiplier)]
	pub type AvgNextFeeMultiplier<T: Config> = StorageValue<_, u128, ValueQuery>;

	// Scheduled calls to be executed, indexed by block number that they should be executed on.
	#[pallet::storage]
	pub type ScheduledCallQueue<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<Option<ScheduleInfo<T>>>, ValueQuery>;

	// Scheduled calls halted due to insufficient funds, indexed by the dispatch owner
	#[pallet::storage]
	pub type HaltedQueue<T: Config> =
		StorageMap<_, Twox64Concat, ScheduleHash<T>, ScheduleInfo<T>, OptionQuery>;

	// Tracks the funds locked in by users to prepay/top-up their scheduled calls
	#[pallet::storage]
	#[pallet::getter(fn scheduled_locked_funds_balances)]
	pub type ScheduleLockedFundBalances<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	// Keeps track of the user balance for the scheduled calls
	#[pallet::storage]
	#[pallet::getter(fn schedule_prepay_balances)]
	pub type SchedulePrepayBalances<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// execute the scheduled tasks
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// schedule logic
			let limit = T::MaximumWeight::get();

			let mut queued = ScheduledCallQueue::<T>::take(now)
				.into_iter()
				.enumerate()
				.filter_map(|(index, s)| Some((index as u32, s?)))
				.collect::<Vec<_>>();

			if queued.len() as u32 > T::MaxScheduledPerBlock::get() {
				log::warn!(
					target: "runtime::scheduler",
					"Warning: This block has more items queued in Scheduler than \
					expected from the runtime configuration. An update might be needed."
				);
			}

			queued.sort_by_key(|(_, s)| s.priority);
			let next = now + One::one();

			let mut total_weight: Weight = 0;
			for (order, (index, mut s)) in queued.into_iter().enumerate() {
				let periodic = s.maybe_periodic.is_some();
				let call_weight = s.call.get_dispatch_info().weight;
				// let mut item_weight = T::WeightInfo::item(periodic, named, Some(resolved));
				let origin =
					<<T as Config>::Origin as From<T::PalletsOrigin>>::from(s.origin.clone())
						.into();
				if ensure_signed(origin).is_ok() {
					// Weights of Signed dispatches expect their signing account to be whitelisted.
					// item_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				}
				// We allow a scheduled call if any is true:
				// - It's priority is `HARD_DEADLINE`
				// - It does not push the weight past the limit.
				// - It is the first item in the schedule
				let hard_deadline = s.priority <= schedule::HARD_DEADLINE;
				let test_weight = total_weight.saturating_add(call_weight);
				if !hard_deadline && order > 0 && test_weight > limit {
					// Cannot be scheduled this block - postpone until next.
					ScheduledCallQueue::<T>::append(next, Some(s));
					continue
				}

				let dispatch_origin = s.origin.clone().into();
				let (maybe_actual_call_weight, result) = match s.call.dispatch(dispatch_origin) {
					Ok(post_info) => (post_info.actual_weight, Ok(())),
					// If the dispatch returned an insufficient balance error, pause the scheduled
					// call and place it in the halt queue
					Err(post_error)
						if post_error.error ==
							traits::fee::InvalidFeeDispatch::InsufficientBalance =>
					{
						// Place the scheduled call into the halted queue until recharging it /
						// before expiry
						HaltedQueue::<T>::insert(&s.id, s);
						// TODO: discuss with the team and decide on a good expiry duration. For
						// now, I'm setting it to be 30 blocks At the time of expiry, the halted
						// call is removed permanently from the HaltedQueue
						DeathBowl::<T>::append(now + 30 * One::one(), s.id);
						continue
					},
					Err(error_and_info) => (None, Err(error_and_info.error)),
				};
				let actual_call_weight = maybe_actual_call_weight.unwrap_or(call_weight);
				// total_weight.saturating_accrue(item_weight);
				// total_weight.saturating_accrue(actual_call_weight);

				Self::deposit_event(Event::Dispatched {
					task: (now, index),
					id: s.id.clone(),
					result,
				});

				if let &Some((period, count)) = &s.maybe_periodic {
					if count > 1 {
						s.maybe_periodic = Some((period, count - 1));
					} else {
						s.maybe_periodic = None;
					}
					let wake = now + period;
					ScheduledCallQueue::<T>::append(wake, Some(s));
				}
			}
			total_weight
		}
	}

	// perform bookkeeping
	// fn on_finalize(now: T::BlockNumber) {
	// 	let current_fee_multipler = NextFeeMultiplier::<T>::get().into_inner();
	// 	let running_avg: u128 = unsafe {
	// 		current_fee_multipler.saturating_div(now as u128).saturating_add(
	// 			Self::avg_next_fee_multiplier()
	// 				.saturating_mul((now as u128 - 1u128).saturating_div(now as u128)),
	// 		)
	// 	};
	// 	AvgNextFeeMultiplier::<T>::put(running_avg);
	// }
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000_000)]
		pub fn schedule_call(
			origin: OriginFor<T>,
			when: T::BlockNumber,
			call: Box<<T as pallet::Config>::Call>,
			maybe_periodic: Option<schedule::Period<T::BlockNumber>>,
			priority: schedule::Priority,
		) -> DispatchResult {
			T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let origin = <T as Config>::Origin::from(origin);
			// Dereference the call
			let call = *call;
			// generate a unique identity to the scheduled call, i.e., hash
			let hash = <T as frame_system::Config>::Hashing::hash_of(&call);
			let when = Self::resolve_time(DispatchTime::At(when))?;

			// sanitize maybe_periodic
			let maybe_periodic = maybe_periodic
				.filter(|p| p.1 > 1 && !p.0.is_zero())
				// Remove one from the number of repetitions since we will schedule one now.
				.map(|(p, c)| (p, c - 1));
			let s = Some(Scheduled {
				id: hash,
				priority,
				call,
				maybe_periodic,
				origin,
				_phantom: PhantomData::<T::AccountId>::default(),
			});

			// ScheduledCallQueue::<T>::append(when, s);
			Ok(())
		}
		#[pallet::weight(1000_000)]
		pub fn schedule_call_exec(
			origin: OriginFor<T>,
			call: Box<<T as pallet::Config>::Call>,
		) -> DispatchResult {
			T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let origin: T::PalletsOrigin = <T as Config>::Origin::from(origin).caller().clone();
			let dispatch_origin = origin.clone().into();
			match call.dispatch(dispatch_origin) {
				Ok(_) => Ok(()),
				Err(_) => Err(DispatchError::Other("Scheduled call dispatch error")),
			}
		}

		#[pallet::weight(1000_000)]
		pub fn resume_halted_call(origin: OriginFor<T>) -> DispatchResult {
			let from = ensure_signed(origin)?;
			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn top_up_locked_fund_balance(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			T::MultiCurrency::transfer(
				T::NativeAssetId::get(),
				&from,
				&T::ScheduleLockedFundAccountId::get(),
				amount,
			)?;
			// Update the user's schedule locked funds details
			let current_locked_funds = Self::scheduled_locked_funds_balances(from.clone());
			ScheduleLockedFundBalances::<T>::insert(&from, current_locked_funds + amount);
			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn top_up_prepay_balance(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let from = ensure_signed(origin)?;
			T::MultiCurrency::transfer(
				T::NativeAssetId::get(),
				&from,
				&T::SchedulePrepayAccountId::get(),
				amount,
			)?;
			// Update the user's schedule prepay balance details
			let current_prepay_balance = Self::schedule_prepay_balances(from.clone());
			SchedulePrepayBalances::<T>::insert(&from, current_prepay_balance + amount);
			Ok(())
		}
	}
}
// Some internal functions
impl<T: Config> Pallet<T> {
	fn resolve_time(when: DispatchTime<T::BlockNumber>) -> Result<T::BlockNumber, DispatchError> {
		let now = frame_system::Pallet::<T>::block_number();

		let when = match when {
			DispatchTime::At(x) => x,
			DispatchTime::After(x) => now.saturating_add(x).saturating_add(One::one()),
		};
		if when <= now {
			return Err(Error::<T>::TargetBlockNumberInPast.into())
		}
		Ok(when)
	}
}
