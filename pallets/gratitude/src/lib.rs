#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

//pub const GRATITUDE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::GratitudeToken);

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, BoundedVec};
	use frame_system::{ensure_signed, pallet_prelude::OriginFor};
	use orml_traits::{MultiCurrency, MultiCurrencyExtended};
	use primitives::CurrencyId;

	type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::storage]
	pub(super) type GratitudeTrail<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, /* who */
		Blake2_128Concat,
		T::BlockNumber,                                     /* when */
		(BalanceOf<T>, BoundedVec<u8, T::MaxReasonLength>), /* how much and why */
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GratitudeAccepted {
			from: T::AccountId,
			amount: BalanceOf<T>,
			reason: BoundedVec<u8, T::MaxReasonLength>,
		},
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + orml_tokens::Config {
		type MultiCurrency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Call: From<Call<Self>>;

		#[pallet::constant]
		type GratitudeAccountId: Get<Self::AccountId>;

		#[pallet::constant]
		type MaxReasonLength: Get<u32>;

		#[pallet::constant]
		type GratitudeCurrency: Get<CurrencyId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000_000)]
		pub fn tip(
			origin: OriginFor<T>,
			tip: BalanceOf<T>,
			reason: BoundedVec<u8, T::MaxReasonLength>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			T::MultiCurrency::transfer(
				T::GratitudeCurrency::get(),
				&from,
				&T::GratitudeAccountId::get(),
				tip,
			)?;

			let current_block = <frame_system::Pallet<T>>::block_number();
			<GratitudeTrail<T>>::insert(&from, &current_block, (tip, reason.clone()));

			Self::deposit_event(Event::GratitudeAccepted { from, amount: tip, reason });

			Ok(())
		}
	}
}
