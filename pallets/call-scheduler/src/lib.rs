//# Call Scheduler
//# this module provides call scheduling funtionalities

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::{Codec, Decode, Encode};
use frame_support::{
	dispatch::{DispatchError, DispatchResult, Dispatchable, Parameter, PostDispatchInfo},
	pallet_prelude::*,
	sp_runtime::{
		traits::{BadOrigin, One, Saturating, Zero},
		RuntimeDebug,
	},
	traits::{
		schedule::{self, DispatchTime, LookupError, MaybeHashed},
		EnsureOrigin, Get, IsType, OriginTrait, StorageVersion,
	},
	weights::{GetDispatchInfo, Weight},
};
use frame_system::{self, ensure_signed, pallet_prelude::*};
use orml_traits::{arithmetic::CheckedAdd, MultiCurrency};
pub use pallet::*;
use primitives::CurrencyId;
use scale_info::TypeInfo;
use sp_std::{marker::PhantomData, prelude::*};

/// Just a simple index for naming period tasks.
pub type PeriodicIndex = u32;
/// The location of a scheduled task that can be used to remove it.
pub type TaskAddress<BlockNumber> = (BlockNumber, u32);

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Information regarding an item to be executed in the future.
#[cfg_attr(any(feature = "std", test), derive(PartialEq, Eq))]
#[derive(Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Scheduled<Call, BlockNumber, PalletsOrigin, AccountId> {
	/// The unique identity for this task, if there is one.
	id: Vec<u8>,
	/// This task's priority.
	priority: schedule::Priority,
	/// The call to be dispatched.
	call: Call,
	/// If the call is periodic, then this points to the information concerning that.
	maybe_periodic: Option<schedule::Period<BlockNumber>>,
	/// The origin to dispatch the call.
	origin: PalletsOrigin,
	retry_count: u32,
	error_count: u32,
	_phantom: PhantomData<AccountId>,
}

pub type ScheduledType<T> = Scheduled<
	<T as Config>::Call,
	<T as frame_system::Config>::BlockNumber,
	<T as Config>::PalletsOrigin,
	<T as frame_system::Config>::AccountId,
>;

#[frame_support::pallet]
pub mod pallet {
	use primitives::Amount;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// `system::Config` should always be included in our implied traits.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The aggregated origin which the dispatch will take.
		type Origin: OriginTrait<PalletsOrigin = Self::PalletsOrigin>
			+ From<Self::PalletsOrigin>
			+ IsType<<Self as frame_system::Config>::Origin>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>
			+ Codec
			+ Clone
			+ Eq
			+ TypeInfo;

		/// The aggregated call type.
		type Call: Parameter
			+ Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>;
		/// Multicurrency
		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;
		/// The maximum weight that may be scheduled per block for any dispatchables of less
		/// priority than `schedule::HARD_DEADLINE`.
		#[pallet::constant]
		type MaximumWeight: Get<Weight>;

		/// Required origin to schedule or cancel calls.
		type ScheduleOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		#[pallet::constant]
		type NativeAssetId: Get<CurrencyId>;

		// The account that pays for the scheduled calls, this balance can be topped up from the
		// locked funds from ScheduleReserve
		#[pallet::constant]
		type SchedulePrepayAccountId: Get<Self::AccountId>;

		// The Account where users lock funds for prepaying their scheduled calls
		#[pallet::constant]
		type ScheduleLockedFundAccountId: Get<Self::AccountId>;

		/// The maximum number of scheduled calls in the queue for a single block.
		/// Not strictly enforced, but used for weight estimation.
		#[pallet::constant]
		type MaxScheduledPerBlock: Get<u32>;

		#[pallet::constant]
		type MaxScheduledCallRetries: Get<u32>;

		#[pallet::constant]
		type MaxScheduledCallErrors: Get<u32>;

		// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;
	}

	/// Items to be executed, indexed by the block number that they should be executed on.
	#[pallet::storage]
	pub type ScheduledCallQueue<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<Option<ScheduledType<T>>>, ValueQuery>;

	/// Lookup from identity to the block number and index of the task.
	#[pallet::storage]
	pub(crate) type Lookup<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, TaskAddress<T::BlockNumber>>;

	/// Tracks who can redeem their scheduled call prepaid fee.
	#[pallet::storage]
	#[pallet::getter(fn check_redeem_scheduled_call_fee)]
	pub(crate) type RedeemScheduledCallFee<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, bool, OptionQuery>;

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

	/// Events type.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Scheduled some task.
		Scheduled { when: T::BlockNumber, index: u32 },
		/// Canceled some task.
		Canceled { when: T::BlockNumber, index: u32 },
		/// Dispatched some task.
		Dispatched { task: TaskAddress<T::BlockNumber>, id: Vec<u8>, result: DispatchResult },
		/// The call for the provided hash was not found so the task has been aborted.
		CallLookupFailed { task: TaskAddress<T::BlockNumber>, id: Vec<u8>, error: LookupError },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to schedule a call
		FailedToSchedule,
		/// Cannot find the scheduled call.
		NotFound,
		/// Given target block number is in the past.
		TargetBlockNumberInPast,
		/// Reschedule failed because it does not change scheduled time.
		RescheduleNoChange,
		/// Scheduled call is still active, so cannot redeem the fee.
		ScheduledCallStillActiveOrNone,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Execute the scheduled calls
		fn on_initialize(now: T::BlockNumber) -> Weight {
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
				// Remove the call from the lookup table
				Lookup::<T>::remove(s.id.clone());
				let call_weight = s.call.get_dispatch_info().weight;
				// let mut item_weight = T::WeightInfo::item(periodic, named, Some(resolved));
				let origin =
					<<T as Config>::Origin as From<T::PalletsOrigin>>::from(s.origin.clone())
						.into();
				if ensure_signed(origin.clone()).is_ok() {
					// Weights of Signed dispatches expect their signing account to be whitelisted.
					total_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				}

				// We allow a scheduled call if any is true:
				// - It's priority is `HARD_DEADLINE`
				// - It does not push the weight past the limit.
				// - It is the first item in the schedule
				let hard_deadline = s.priority <= schedule::HARD_DEADLINE;
				let test_weight = total_weight.saturating_add(call_weight);
				if !hard_deadline && order > 0 && test_weight > limit {
					if s.retry_count >= T::MaxScheduledCallRetries::get() {
						// Refund the scheduled call prepaid balance back to the origin, if the
						// refund somehow fails then schedule the call for the next block and retry
						// the refund. Get the current prepay balance of the origin
						let origin_account = ensure_signed(origin.clone()).unwrap();
						RedeemScheduledCallFee::<T>::insert(&origin_account, true);
						continue
					} else {
						s.retry_count += 1;
						let id = s.id.clone();
						ScheduledCallQueue::<T>::mutate(next, |queue| queue.push(Some(s)));
						let index = ScheduledCallQueue::<T>::decode_len(next).unwrap_or(0);
						Lookup::<T>::insert(id, (next, index as u32));
						continue
					}
				}

				let dispatch_origin = s.origin.clone().into();
				let (maybe_actual_call_weight, dispatch_result) =
					match s.call.clone().dispatch(dispatch_origin) {
						Ok(post_info) => (post_info.actual_weight, Ok(())),
						Err(error_and_info) => {
							s.error_count += 1;
							(error_and_info.post_info.actual_weight, Err(error_and_info.error))
						},
					};

				let actual_call_weight = maybe_actual_call_weight.unwrap_or(call_weight);
				// total_weight.saturating_accrue(item_weight);
				total_weight.saturating_accrue(actual_call_weight);

				Self::deposit_event(Event::Dispatched {
					task: (now, index),
					id: s.id.clone(),
					result: dispatch_result.clone(),
				});

				if let &Some((period, count)) = &s.maybe_periodic {
					if let Err(_) = dispatch_result {
						if s.error_count >= T::MaxScheduledCallErrors::get() {
							// Refund the remaining (if any) scheduled call prepaid balance back to
							// the origin.
							let origin_account = ensure_signed(origin.clone()).unwrap();
							RedeemScheduledCallFee::<T>::insert(&origin_account, true);
						}
					} else {
						// Reschedule the call if there was no threshold number of errors
						if count > 1 {
							s.maybe_periodic = Some((period, count - 1));
						} else {
							s.maybe_periodic = None;
						}
						let wake = now + period;
						// If scheduled is named, place its information in `Lookup`
						let wake_index = ScheduledCallQueue::<T>::decode_len(wake).unwrap_or(0);
						Lookup::<T>::insert(s.id.clone(), (wake, wake_index as u32));
						ScheduledCallQueue::<T>::mutate(wake, |queue| queue.push(Some(s)));
					}
				}
			}
			total_weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Anonymously schedule a task.
		#[pallet::weight(1000_000)]
		pub fn schedule_call(
			origin: OriginFor<T>,
			when: T::BlockNumber,
			call: Box<<T as pallet::Config>::Call>,
			id: Vec<u8>,
			maybe_periodic: Option<schedule::Period<T::BlockNumber>>,
			priority: schedule::Priority,
		) -> DispatchResult {
			T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let origin = <T as Config>::Origin::from(origin).caller().clone();
			// Dereference the call
			let call = *call;
			let when = Self::resolve_time(DispatchTime::At(when))?;

			// sanitize maybe_periodic
			let maybe_periodic = maybe_periodic
				.filter(|p| p.1 > 1 && !p.0.is_zero())
				// Remove one from the number of repetitions since we will schedule one now.
				.map(|(p, c)| (p, c - 1));

			let s = Scheduled {
				id: id.clone(),
				priority,
				call,
				maybe_periodic,
				origin,
				retry_count: 0u32,
				error_count: 0u32,
				_phantom: PhantomData::<T::AccountId>::default(),
			};

			ScheduledCallQueue::<T>::mutate(when, |queue| queue.push(Some(s)));
			let index = ScheduledCallQueue::<T>::decode_len(when).unwrap_or(1) as u32 - 1;
			let address = (when, index);
			Lookup::<T>::insert(&id, &address);
			Self::deposit_event(Event::Scheduled { when, index });

			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn schedule_call_exec(
			origin: OriginFor<T>,
			call: Box<<T as pallet::Config>::Call>,
		) -> DispatchResult {
			T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let origin: T::PalletsOrigin = <T as Config>::Origin::from(origin).caller().clone();
			// let dispatch_origin = origin.clone().into();
			match call.dispatch(origin.into()) {
				Ok(_) => Ok(()),
				Err(_) => Err(DispatchError::Other("Scheduled call dispatch error")),
			}
		}

		#[pallet::weight(1000_000)]
		pub fn redeem_schedule_fee(origin: OriginFor<T>) -> DispatchResult {
			let from = ensure_signed(origin)?;
			if let None | Some(false) = Self::check_redeem_scheduled_call_fee(from.clone()) {
				return Err(Error::<T>::ScheduledCallStillActiveOrNone.into())
			}
			// Get the total redeemable amount
			let redeem_balance = Self::schedule_prepay_balances(from.clone());

			T::MultiCurrency::transfer(
				T::NativeAssetId::get(),
				&T::SchedulePrepayAccountId::get(),
				&T::ScheduleLockedFundAccountId::get(),
				redeem_balance,
			)?;
			// If the refund is successful then reset the origin's balance to 0.
			SchedulePrepayBalances::<T>::remove(from.clone());
			// Increase the ScheduleLockedFundBalances of the origin by the refunded
			// amount
			ScheduleLockedFundBalances::<T>::mutate(from.clone(), |balance| {
				*balance = balance
					.checked_add(&redeem_balance)
					.expect("overflow when updating locked fund balance")
			});
			// After redeeming the balance, "toggle the redemption switch"
			RedeemScheduledCallFee::<T>::remove(from);
			Ok(())
		}

		/// Cancel a named scheduled task.
		#[pallet::weight(1000_000)]
		pub fn cancel_schedule_call(origin: OriginFor<T>, id: Vec<u8>) -> DispatchResult {
			T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let from = ensure_signed(origin.clone())?;
			let origin = <T as Config>::Origin::from(origin).caller().clone();
			Lookup::<T>::try_mutate_exists(id, |lookup| -> DispatchResult {
				if let Some((when, index)) = lookup.take() {
					let i = index as usize;
					ScheduledCallQueue::<T>::try_mutate(when, |queue| -> DispatchResult {
						if let Some(s) = queue.get_mut(i) {
							if s.as_ref().unwrap().origin != origin {
								return Err(BadOrigin.into())
							}
							*s = None;
						}
						Ok(())
					})?;
					Self::deposit_event(Event::Canceled { when, index });
					// Mark the cancelled call's origin as redeemable
					RedeemScheduledCallFee::<T>::insert(&from, true);
					Ok(())
				} else {
					return Err(Error::<T>::NotFound.into())
				}
			})
		}

		#[pallet::weight(1000_000)]
		pub fn fund_scheduled_call(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let from = ensure_signed(origin)?;
			T::MultiCurrency::transfer(
				T::NativeAssetId::get(),
				&from,
				&T::ScheduleLockedFundAccountId::get(),
				amount,
			)?;
			// Update the balance in the storage
			ScheduleLockedFundBalances::<T>::mutate(from.clone(), |balance| {
				*balance = balance
					.checked_add(&amount)
					.expect("overflow when updating locked fund balance")
			});

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
