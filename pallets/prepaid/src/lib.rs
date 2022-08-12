//! ## prepaid token module
//!
//! by prepaying native token, user receive prepaid token which can be used for paying fee's with a
//! premium rate. currently to avoid stale circulation for native token, there's an threshold
//! allowed for prepaid tokens.
//!
//! the prepaying path is as follows:
//! 1. check the resulted issuance didn't exceed the threshold.
//! 2. move balances from user to the PalletId controlled account and reserved
//! 3. issue new prepaid token for user
//! 4. when paid using prepaid token, the tokens then are unreserved and sent to the targted account

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{pallet_prelude::*, traits::fungibles};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use sp_runtime::{
	traits::{AccountIdConversion, Saturating},
	FixedPointNumber, FixedPointOperand,
};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T, C> = <C as MultiCurrency<AccountIdOf<T>>>::Balance;
type CurrencyOf<T, C> = <C as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::PalletId;
	use orml_traits::MultiReservableCurrency;
	use sp_runtime::FixedU128;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// max ratio of total circulation of native token allowed to be prepaid.
		type MaxPrepaidRaio: Get<FixedU128>;

		type MultiCurrency: MultiReservableCurrency<AccountIdOf<Self>>
			+ fungibles::Transfer<AccountIdOf<Self>>
			+ fungibles::Inspect<
				AccountIdOf<Self>,
				AssetId = CurrencyOf<Self, Self::MultiCurrency>,
				Balance = BalanceOf<Self, Self::MultiCurrency>,
			>;

		type NativeCurrencyId: Get<CurrencyOf<Self, Self::MultiCurrency>>;
		type PrepaidCurrencyId: Get<CurrencyOf<Self, Self::MultiCurrency>>;

		type PalletId: Get<PalletId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		Aquired { amount: BalanceOf<T, T::MultiCurrency> },
		Redeemed { amount: BalanceOf<T, T::MultiCurrency> },
	}

	#[pallet::error]
	pub enum Error<T> {
		MaxPrepaidExceeded,
		InsufficientAmount,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		BalanceOf<T, T::MultiCurrency>: FixedPointOperand,
	{
		#[pallet::weight(1000)]
		pub fn prepaid_native(
			origin: OriginFor<T>,
			amount: BalanceOf<T, T::MultiCurrency>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::prepaid(who, amount)?;

			Self::deposit_event(Event::<T>::Aquired { amount });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	BalanceOf<T, T::MultiCurrency>: FixedPointOperand,
{
	/// prepaid native token by reserving,  get amount in prepaid token
	fn prepaid(who: AccountIdOf<T>, amount: BalanceOf<T, T::MultiCurrency>) -> DispatchResult {
		let total_native_issued =
			<T::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::total_issuance(
				T::NativeCurrencyId::get(),
			);

		let threshold_amount = T::MaxPrepaidRaio::get().saturating_mul_int(total_native_issued);

		let current_prepaid_issued =
			<T::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::total_issuance(
				T::PrepaidCurrencyId::get(),
			);

		// early return if it's not possible to prepaid more
		if current_prepaid_issued.saturating_add(amount) >= threshold_amount {
			return Err(From::from(Error::<T>::MaxPrepaidExceeded))
		}

		let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account();

		<T::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::transfer(
			T::NativeCurrencyId::get(),
			&who,
			&pallet_account,
			amount,
		)?;

		// early return if user didn't have enough balance to reserve
		if !<T::MultiCurrency as MultiReservableCurrency<AccountIdOf<T>>>::can_reserve(
			T::NativeCurrencyId::get(),
			&pallet_account,
			amount,
		) {
			return Err(From::from(Error::<T>::InsufficientAmount))
		}

		<T::MultiCurrency as MultiReservableCurrency<AccountIdOf<T>>>::reserve(
			T::NativeCurrencyId::get(),
			&pallet_account,
			amount,
		)?;

		<T::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::deposit(
			T::PrepaidCurrencyId::get(),
			&who,
			amount,
		)?;

		Ok(())
	}

	/// unserve native token after prepaid token is used
	pub fn unserve_to(
		receiver: AccountIdOf<T>,
		amount: BalanceOf<T, T::MultiCurrency>,
	) -> DispatchResult {
		let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account();

		<T::MultiCurrency as MultiReservableCurrency<AccountIdOf<T>>>::unreserve(
			T::NativeCurrencyId::get(),
			&pallet_account,
			amount,
		);

		let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account();

		<T::MultiCurrency as fungibles::Transfer<AccountIdOf<T>>>::transfer(
			T::NativeCurrencyId::get(),
			&pallet_account,
			&receiver,
			amount,
			false,
		)?;

		Ok(())
	}
}
