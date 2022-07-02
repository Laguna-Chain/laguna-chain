#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::{DisptachError, DispatchResult, Dispatchable, Parameter}, pallet_prelude::*, weights::{GetDispatchInfo, Weight}, traits::{
    schedule::{self, DispatchTime},
    EnsureOrigin, Get, IsType, OriginTrait, PalletInfoAccess, PrivilegeCmp, StorageVersion,
}};
use frame_system::{ensure_signed, pallet_prelude::OriginFor};
use sp_runtime::{
	traits::{BadOrigin, One, Saturating, Zero},
	RuntimeDebug,
};
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, TokenId};
use scale_info::TypeInfo;
use primitives::CurrencyId;

pub use pallet::*;

/// Just a simple index for naming period tasks.
pub type PeriodicIndex = u32;
/// The location of a scheduled task that can be used to remove it.
pub type TaskAddress<BlockNumber> = (BlockNumber, u32);
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<T as frame_system::Config>::AccountId,>>::Balance;

/// Information regarding an item to be executed in the future.
#[cfg_attr(any(feature = "std", test), derive(PartialEq, Eq))]
#[derive(Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Scheduled<Call, BlockNumber, PalletsOrigin, AccountId> {
	/// The unique identity for this task, if there is one.
	maybe_id: Option<Vec<u8>>,
	/// This task's priority.
	priority: schedule::Priority,
	/// The call to be dispatched.
	call: Call,
	/// If the call is periodic, then this points to the information concerning that.
	maybe_periodic: Option<schedule::Period<BlockNumber>>,
	/// The origin to dispatch the call.
	origin: PalletsOrigin,
	_phantom: PhantomData<AccountId>,
}

pub(crate) trait MarginalWeightInfo: WeightInfo {
	fn item(periodic: bool, named: bool, resolved: Option<bool>) -> Weight {
		match (periodic, named, resolved) {
			(_, false, None) => Self::on_initialize_aborted(2) - Self::on_initialize_aborted(1),
			(_, true, None) =>
				Self::on_initialize_named_aborted(2) - Self::on_initialize_named_aborted(1),
			(false, false, Some(false)) => Self::on_initialize(2) - Self::on_initialize(1),
			(false, true, Some(false)) =>
				Self::on_initialize_named(2) - Self::on_initialize_named(1),
			(true, false, Some(false)) =>
				Self::on_initialize_periodic(2) - Self::on_initialize_periodic(1),
			(true, true, Some(false)) =>
				Self::on_initialize_periodic_named(2) - Self::on_initialize_periodic_named(1),
			(false, false, Some(true)) =>
				Self::on_initialize_resolved(2) - Self::on_initialize_resolved(1),
			(false, true, Some(true)) =>
				Self::on_initialize_named_resolved(2) - Self::on_initialize_named_resolved(1),
			(true, false, Some(true)) =>
				Self::on_initialize_periodic_resolved(2) - Self::on_initialize_periodic_resolved(1),
			(true, true, Some(true)) =>
				Self::on_initialize_periodic_named_resolved(2) -
					Self::on_initialize_periodic_named_resolved(1),
		}
	}
}
impl<T: WeightInfo> MarginalWeightInfo for T {}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// # call-scheduler
/// 
/// this module provides functionality for call scheduling 
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    /// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

    #[pallet::config]
    pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

    /// The aggregated call type.
	type Call: Parameter
        + Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
        + GetDispatchInfo
        + From<system::Call<Self>>;

    /// The caller origin, overarching type of all pallets origins.
    type PalletsOrigin: From<system::RawOrigin<Self::AccountId>> + Codec + Clone + Eq + TypeInfo;

    /// The aggregated origin which the dispatch will take.
    type Origin: OriginTrait<PalletsOrigin = Self::PalletsOrigin>
        + From<Self::PalletsOrigin>
        + IsType<<Self as system::Config>::Origin>;

    /// The maximum weight that may be scheduled per block for any dispatchables of less
	/// priority than `schedule::HARD_DEADLINE`.
    #[pallet::constant]
	type MaximumWeight: Get<Weight>;

    /// The maximum number of scheduled calls in the queue for a single block.
	/// Not strictly enforced, but used for weight estimation.
	#[pallet::constant]
    type MaxScheduledPerBlock: Get<u32>;

    #[pallet::constant]
    type ReserveAccount: Get<Self::AccountId>;

    #[pallet::constant]
    type LockedFundsAccount: Get<Self::AccountId>;

    #[pallet::constant]
    type NativeCurrencyId: Get<CurrencyId>;

    type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;

    }
    
    #[pallet::storage]
    #[pallet::getter(fn reserve_balance_per_account)]
    pub type ReserveBalancePerAccount<T: Config> = 
        StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Items to be executed, indexed by the block number that they should be executed on.
	#[pallet::storage]
	pub type Agenda<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<Option<ScheduledV3Of<T>>>, ValueQuery>;

    /// Lookup from identity to the block number and index of the task.
	#[pallet::storage]
	pub(crate) type Lookup<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, TaskAddress<T::BlockNumber>>;
    
    #[pallet::storage]
    #[pallet::getter(fn treasury_balance_per_account)]
    pub type LockedFundsBalancePerAccount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Event type
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Scheduled some task.
        Scheduled { when: T::BlockNumber, index: u32},
        /// Canceled some task.
        Canceled { when: T::BlockNumber, index: u32},
        /// Halted some task
        Halted { when: T::BlockNumber, index: u32},
        /// Dispatched some task.
		Dispatched {
			task: TaskAddress<T::BlockNumber>,
			id: Option<Vec<u8>>,
			result: DispatchResult,
		},
    }

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Execute the scheduled calls
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let limit = T::MaximumWeight::get();

			let mut queued = Agenda::<T>::take(now)
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

            let mut total_weight: Weight = T::WeightInfo::on_initialize(0);

            for (order, (index, mut s)) in queued.into_iter().enumerate() {
				let named = if let Some(ref id) = s.maybe_id {
					Lookup::<T>::remove(id);
					true
				} else {
					false
				};

            let periodic = s.maybe_periodic.is_some();
			let call_weight = call.get_dispatch_info().weight;
            let mut item_weight = T::WeightInfo::item(periodic, named, Some(resolved));
			let origin =
				<<T as Config>::Origin as From<T::PalletsOrigin>>::from(s.origin.clone())
				.into();
            if ensure_signed(origin).is_ok() {
				// Weights of Signed dispatches expect their signing account to be whitelisted.
				item_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
			}
            // We allow a scheduled call if any is true:
				// - It's priority is `HARD_DEADLINE`
				// - It does not push the weight past the limit.
				// - It is the first item in the schedule
				let hard_deadline = s.priority <= schedule::HARD_DEADLINE;
				let test_weight =
					total_weight.saturating_add(call_weight).saturating_add(item_weight);
				if !hard_deadline && order > 0 && test_weight > limit {
					// Cannot be scheduled this block - postpone until next.
					total_weight.saturating_accrue(T::WeightInfo::item(false, named, None));
					if let Some(ref id) = s.maybe_id {
						// NOTE: We could reasonably not do this (in which case there would be one
						// block where the named and delayed item could not be referenced by name),
						// but we will do it anyway since it should be mostly free in terms of
						// weight and it is slightly cleaner.
						let index = Agenda::<T>::decode_len(next).unwrap_or(0);
						Lookup::<T>::insert(id, (next, index as u32));
					}
					Agenda::<T>::append(next, Some(s));
					continue
				}

                let dispatch_origin = s.origin.clone().into();
				let (maybe_actual_call_weight, result) = match call.dispatch(dispatch_origin) {
					Ok(post_info) => (post_info.actual_weight, Ok(())),
					Err(error_and_info) =>
						(error_and_info.post_info.actual_weight, Err(error_and_info.error)),
				};
				let actual_call_weight = maybe_actual_call_weight.unwrap_or(call_weight);
				total_weight.saturating_accrue(item_weight);
				total_weight.saturating_accrue(actual_call_weight);

				Self::deposit_event(Event::Dispatched {
					task: (now, index),
					id: s.maybe_id.clone(),
					result,
				});

				if let &Some((period, count)) = &s.maybe_periodic {
					if count > 1 {
						s.maybe_periodic = Some((period, count - 1));
					} else {
						s.maybe_periodic = None;
					}
					let wake = now + period;
					// If scheduled is named, place its information in `Lookup`
					if let Some(ref id) = s.maybe_id {
						let wake_index = Agenda::<T>::decode_len(wake).unwrap_or(0);
						Lookup::<T>::insert(id, (wake, wake_index as u32));
					}
					Agenda::<T>::append(wake, Some(s));
				}
			}
			total_weight
        }

        fn on_finalize(when: T::BlockNumber) {
            // get the TargetFeeAdjustment for the current block and update the average with the latest value
            // get the current baseFee and update the average
        } 
    }

   

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// prepay / lock funds to be later used for scheduling
        #[pallet::weight(1000_000_000)]
        pub fn lock_funds(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            let current_locked_funds = Self::locked_funds_balance_per_account(&from);

			T::MultiCurrency::transfer(T::NativeCurrencyId::get(), &from, &T::LockedFundsAccount::get(), amount)?;

			let updated_locked_funds =
				current_locked_funds.checked_add(&amount).unwrap();

			<LockedFundsBalancePerAccount<T>>::insert(&from, updated_locked_funds);

        }
        /// schedule a call
        #[pallet::weight(1000_000_000)]
        pub fn schedule_call(origin: OriginFor<T>, when: T::BlockNumber, maybe_periodic: Option<schedule::Period<T::BlockNumber>>, priority: schedule::Priority, call: Box<T::Call>) -> DispatchResult{
            T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let origin = <T as Config>::Origin::from(origin);
            let when = Self::resolve_time(DispatchTime::At(when))?;

            // sanitize maybe_periodic
            let maybe_periodic = maybe_periodic
                .filter(|p| p.1 > 1 && !p.0.is_zero())
                // Remove one from the number of repetitions since we will schedule one now.
                .map(|(p, c)| (p, c - 1));

            let s = Some(Scheduled {
                maybe_id: None,
                priority,
                *call,
                maybe_periodic,
                origin,
                _phantom: PhantomData::<T::AccountId>::default(),
            });
            Agenda::<T>::append(when, s);
            let index = Agenda::<T>::decode_len(when).unwrap_or(1) as u32 - 1;
            Self::deposit_event(Event::Scheduled { when, index });

            Ok((when, index))
        }
    }

}