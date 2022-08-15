//! ## pallet-currencies
//!
//! This is a unified adapter to expose various currency sources, including
//!
//! 1. native tokens
//! 2. contract-based tokens
//!
//! For single-token use cases, the BasicCurrency* and fungible::* traits are implemented for this
//! pallet. It's pointing to the NativeCurrencyId specified in Config
//!
//! For multi-token use cases, the MultiCurrency* and fungibles::* traits are implemented for this
//! pallet. When trying to operate on native tokens, the T::MultiCurrency associate type will be
//! used. When trying to operate on contract based tokens, the T::ContractAssets associate type will
//! be used, be cautious that not all features are enabled for contrat based tokens due to their
//! natural differences in design.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::tokens::{fungible, fungibles, DepositConsequence, WithdrawConsequence},
};

use frame_system::pallet_prelude::*;

use orml_traits::{
	currency::TransferAll, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency,
	BasicReservableCurrency, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
	MultiReservableCurrency,
};

pub use pallet::*;
use primitives::CurrencyId;
use sp_core::U256;
use sp_runtime::traits::{CheckedAdd, Convert, Saturating, Zero};
use traits::currencies::TokenAccess;

pub mod adapters;

/// +++++++++++++++++++++++
/// specifying type alises.
/// +++++++++++++++++++++++

type AmountOf<T> = <<T as Config>::MultiCurrency as MultiCurrencyExtended<AccountIdOf<T>>>::Amount;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type CurrencyIdOf<T> = <Pallet<T> as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyId>;

		/// native multi-token adatper
		type MultiCurrency: TransferAll<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ fungibles::Inspect<Self::AccountId, AssetId = CurrencyId, Balance = BalanceOf<Self>>
			+ fungibles::Mutate<Self::AccountId, AssetId = CurrencyId, Balance = BalanceOf<Self>>
			+ fungibles::Transfer<Self::AccountId, AssetId = CurrencyId, Balance = BalanceOf<Self>>
			+ fungibles::Unbalanced<Self::AccountId, AssetId = CurrencyId, Balance = BalanceOf<Self>>
			+ fungibles::InspectHold<Self::AccountId, AssetId = CurrencyId, Balance = BalanceOf<Self>>
			+ fungibles::MutateHold<Self::AccountId, AssetId = CurrencyId, Balance = BalanceOf<Self>>;

		/// contract asset adapter
		type ContractAssets: TokenAccess<Self, Balance = BalanceOf<Self>>;

		/// provide mechanism to get account_id from pub key, used for contract-asset lookup
		type ConvertIntoAccountId: Convert<[u8; 32], Self::AccountId>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Transfer succeeded.
		Transfer { from: AccountIdOf<T>, to: AccountIdOf<T>, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		BalanceTooLow,
		InvalidContractOperation,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		U256: From<BalanceOf<T>>,
	{
		#[pallet::weight(100_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: AccountIdOf<T>,
			currency_id: CurrencyIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			<Self as MultiCurrency<AccountIdOf<T>>>::transfer(
				currency_id,
				&from,
				&to,
				balance.clone(),
			)?;
			Self::deposit_event(Event::Transfer { from, to, amount: balance });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	pub fn free_balance(account: AccountIdOf<T>, asset_id: CurrencyIdOf<T>) -> BalanceOf<T> {
		<Self as MultiCurrency<AccountIdOf<T>>>::free_balance(asset_id, &account)
	}

	pub fn total_balance(account: AccountIdOf<T>, asset_id: CurrencyIdOf<T>) -> BalanceOf<T> {
		<Self as MultiCurrency<AccountIdOf<T>>>::total_balance(asset_id, &account)
	}
}

/// ++++++++++++++++++++++++++++++++++++++++
/// section for defining provider behaviour.
/// ++++++++++++++++++++++++++++++++++++++++

/// when used as single token currency provider, specified T::NativeCurrency will be used
impl<T: Config> BasicCurrency<AccountIdOf<T>> for Pallet<T> {
	type Balance = BalanceOf<T>;

	fn minimum_balance() -> Self::Balance {
		T::MultiCurrency::minimum_balance(T::NativeCurrencyId::get())
	}

	fn total_issuance() -> Self::Balance {
		T::MultiCurrency::total_issuance(T::NativeCurrencyId::get())
	}

	fn total_balance(who: &AccountIdOf<T>) -> Self::Balance {
		T::MultiCurrency::total_balance(T::NativeCurrencyId::get(), who)
	}

	fn free_balance(who: &AccountIdOf<T>) -> Self::Balance {
		T::MultiCurrency::free_balance(T::NativeCurrencyId::get(), who)
	}

	fn ensure_can_withdraw(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		T::MultiCurrency::ensure_can_withdraw(T::NativeCurrencyId::get(), who, amount)
	}

	fn transfer(
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		T::MultiCurrency::transfer(T::NativeCurrencyId::get(), from, to, amount)
	}

	fn deposit(who: &AccountIdOf<T>, amount: Self::Balance) -> sp_runtime::DispatchResult {
		T::MultiCurrency::deposit(T::NativeCurrencyId::get(), who, amount)
	}

	fn withdraw(who: &AccountIdOf<T>, amount: Self::Balance) -> sp_runtime::DispatchResult {
		T::MultiCurrency::withdraw(T::NativeCurrencyId::get(), who, amount)
	}

	fn can_slash(who: &AccountIdOf<T>, value: Self::Balance) -> bool {
		T::MultiCurrency::can_slash(T::NativeCurrencyId::get(), who, value)
	}

	fn slash(who: &AccountIdOf<T>, amount: Self::Balance) -> Self::Balance {
		T::MultiCurrency::slash(T::NativeCurrencyId::get(), who, amount)
	}
}

impl<T: Config> BasicCurrencyExtended<AccountIdOf<T>> for Pallet<T> {
	type Amount = AmountOf<T>;

	fn update_balance(who: &AccountIdOf<T>, by_amount: Self::Amount) -> sp_runtime::DispatchResult {
		T::MultiCurrency::update_balance(T::NativeCurrencyId::get(), who, by_amount)
	}
}

impl<T: Config> BasicLockableCurrency<AccountIdOf<T>> for Pallet<T> {
	type Moment = T::BlockNumber;

	fn set_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		T::MultiCurrency::set_lock(lock_id, T::NativeCurrencyId::get(), who, amount)
	}

	fn extend_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		T::MultiCurrency::extend_lock(lock_id, T::NativeCurrencyId::get(), who, amount)
	}

	fn remove_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
	) -> sp_runtime::DispatchResult {
		T::MultiCurrency::remove_lock(lock_id, T::NativeCurrencyId::get(), who)
	}
}

impl<T: Config> BasicReservableCurrency<AccountIdOf<T>> for Pallet<T> {
	fn can_reserve(who: &AccountIdOf<T>, value: Self::Balance) -> bool {
		T::MultiCurrency::can_reserve(T::NativeCurrencyId::get(), who, value)
	}

	fn slash_reserved(who: &AccountIdOf<T>, value: Self::Balance) -> Self::Balance {
		T::MultiCurrency::slash_reserved(T::NativeCurrencyId::get(), who, value)
	}

	fn reserved_balance(who: &AccountIdOf<T>) -> Self::Balance {
		T::MultiCurrency::reserved_balance(T::NativeCurrencyId::get(), who)
	}

	fn reserve(who: &AccountIdOf<T>, value: Self::Balance) -> sp_runtime::DispatchResult {
		T::MultiCurrency::reserve(T::NativeCurrencyId::get(), who, value)
	}

	fn unreserve(who: &AccountIdOf<T>, value: Self::Balance) -> Self::Balance {
		T::MultiCurrency::unreserve(T::NativeCurrencyId::get(), who, value)
	}

	fn repatriate_reserved(
		slashed: &AccountIdOf<T>,
		beneficiary: &AccountIdOf<T>,
		value: Self::Balance,
		status: orml_traits::BalanceStatus,
	) -> core::result::Result<Self::Balance, DispatchError> {
		T::MultiCurrency::repatriate_reserved(
			T::NativeCurrencyId::get(),
			slashed,
			beneficiary,
			value,
			status,
		)
	}
}

impl<T: Config> fungible::Inspect<AccountIdOf<T>> for Pallet<T> {
	type Balance = BalanceOf<T>;

	fn total_issuance() -> Self::Balance {
		<T::MultiCurrency as fungibles::Inspect<_>>::total_issuance(T::NativeCurrencyId::get())
	}

	fn minimum_balance() -> Self::Balance {
		<T::MultiCurrency as fungibles::Inspect<_>>::minimum_balance(T::NativeCurrencyId::get())
	}

	fn balance(who: &AccountIdOf<T>) -> Self::Balance {
		<T::MultiCurrency as fungibles::Inspect<_>>::balance(T::NativeCurrencyId::get(), who)
	}

	fn reducible_balance(who: &AccountIdOf<T>, keep_alive: bool) -> Self::Balance {
		<T::MultiCurrency as fungibles::Inspect<_>>::reducible_balance(
			T::NativeCurrencyId::get(),
			who,
			keep_alive,
		)
	}

	fn can_deposit(who: &AccountIdOf<T>, amount: Self::Balance, mint: bool) -> DepositConsequence {
		<T::MultiCurrency as fungibles::Inspect<_>>::can_deposit(
			T::NativeCurrencyId::get(),
			who,
			amount,
			mint,
		)
	}

	fn can_withdraw(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		<T::MultiCurrency as fungibles::Inspect<_>>::can_withdraw(
			T::NativeCurrencyId::get(),
			who,
			amount,
		)
	}
}

impl<T: Config> fungible::Mutate<AccountIdOf<T>> for Pallet<T> {
	fn mint_into(who: &AccountIdOf<T>, amount: Self::Balance) -> DispatchResult {
		<T::MultiCurrency as fungibles::Mutate<_>>::mint_into(
			T::NativeCurrencyId::get(),
			who,
			amount,
		)
	}

	fn burn_from(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		<T::MultiCurrency as fungibles::Mutate<_>>::burn_from(
			T::NativeCurrencyId::get(),
			who,
			amount,
		)
	}
}

impl<T: Config> fungible::Transfer<AccountIdOf<T>> for Pallet<T> {
	fn transfer(
		source: &AccountIdOf<T>,
		dest: &AccountIdOf<T>,
		amount: Self::Balance,
		keep_alive: bool,
	) -> Result<Self::Balance, DispatchError> {
		<T::MultiCurrency as fungibles::Transfer<_>>::transfer(
			T::NativeCurrencyId::get(),
			source,
			dest,
			amount,
			keep_alive,
		)
	}
}

impl<T: Config> fungible::Unbalanced<AccountIdOf<T>> for Pallet<T> {
	fn set_balance(who: &AccountIdOf<T>, amount: Self::Balance) -> DispatchResult {
		<T::MultiCurrency as fungibles::Unbalanced<_>>::set_balance(
			T::NativeCurrencyId::get(),
			who,
			amount,
		)
	}

	fn set_total_issuance(amount: Self::Balance) {
		<T::MultiCurrency as fungibles::Unbalanced<_>>::set_total_issuance(
			T::NativeCurrencyId::get(),
			amount,
		)
	}
}

impl<T: Config> fungible::InspectHold<AccountIdOf<T>> for Pallet<T> {
	fn balance_on_hold(who: &AccountIdOf<T>) -> Self::Balance {
		<T::MultiCurrency as fungibles::InspectHold<_>>::balance_on_hold(
			T::NativeCurrencyId::get(),
			who,
		)
	}

	fn can_hold(who: &AccountIdOf<T>, amount: Self::Balance) -> bool {
		<T::MultiCurrency as fungibles::InspectHold<_>>::can_hold(
			T::NativeCurrencyId::get(),
			who,
			amount,
		)
	}
}

impl<T: Config> fungible::MutateHold<AccountIdOf<T>> for Pallet<T> {
	fn hold(who: &AccountIdOf<T>, amount: Self::Balance) -> DispatchResult {
		<T::MultiCurrency as fungibles::MutateHold<_>>::hold(
			T::NativeCurrencyId::get(),
			who,
			amount,
		)
	}

	fn release(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
		best_effort: bool,
	) -> Result<Self::Balance, DispatchError> {
		<T::MultiCurrency as fungibles::MutateHold<_>>::release(
			T::NativeCurrencyId::get(),
			who,
			amount,
			best_effort,
		)
	}

	fn transfer_held(
		source: &AccountIdOf<T>,
		dest: &AccountIdOf<T>,
		amount: Self::Balance,
		best_effort: bool,
		on_held: bool,
	) -> Result<Self::Balance, DispatchError> {
		<T::MultiCurrency as fungibles::MutateHold<_>>::transfer_held(
			T::NativeCurrencyId::get(),
			source,
			dest,
			amount,
			best_effort,
			on_held,
		)
	}
}

/// when used as multi-token currency provider, T::MultiCurrency will be used, noted that
/// contract-based operation is done via T::TokenAccess
impl<T: Config> MultiCurrency<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	type CurrencyId = CurrencyId;
	type Balance = BalanceOf<T>;

	fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
		match currency_id {
			CurrencyId::Erc20(_) => Default::default(),

			CurrencyId::NativeToken(_) => <T::MultiCurrency as fungibles::Inspect<
				AccountIdOf<T>,
			>>::minimum_balance(currency_id),
		}
	}

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		match currency_id {
			CurrencyId::Erc20(addr) => {
				let asset = T::ConvertIntoAccountId::convert(addr);
				T::ContractAssets::total_supply(asset).unwrap_or_default()
			},

			CurrencyId::NativeToken(_) => <T::MultiCurrency as fungibles::Inspect<
				AccountIdOf<T>,
			>>::total_issuance(currency_id),
		}
	}

	fn total_balance(currency_id: Self::CurrencyId, who: &AccountIdOf<T>) -> Self::Balance {
		match currency_id {
			CurrencyId::Erc20(addr) => {
				let asset = T::ConvertIntoAccountId::convert(addr);
				T::ContractAssets::balance_of(asset, who.clone()).unwrap_or_default()
			},

			CurrencyId::NativeToken(_) => <T::MultiCurrency as fungibles::Inspect<
				AccountIdOf<T>,
			>>::total_issuance(currency_id),
		}
	}

	fn free_balance(currency_id: Self::CurrencyId, who: &AccountIdOf<T>) -> Self::Balance {
		match currency_id {
			CurrencyId::NativeToken(_) => T::MultiCurrency::free_balance(currency_id, who),
			CurrencyId::Erc20(addr) => {
				let asset = T::ConvertIntoAccountId::convert(addr);
				T::ContractAssets::balance_of(asset, who.clone()).unwrap_or_default()
			},
		}
	}

	fn ensure_can_withdraw(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		if amount.is_zero() {
			return Ok(())
		}

		match currency_id {
			CurrencyId::NativeToken(_) =>
				T::MultiCurrency::ensure_can_withdraw(currency_id, who, amount),
			CurrencyId::Erc20(addr) => {
				// handle zero withdrawl
				if amount.is_zero() {
					return Ok(())
				}

				let asset = T::ConvertIntoAccountId::convert(addr);
				let balance = T::ContractAssets::balance_of(asset, who.clone()).unwrap_or_default();

				ensure!(balance >= amount, Error::<T>::BalanceTooLow);
				Ok(())
			},
		}
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		match currency_id {
			CurrencyId::NativeToken(_) => T::MultiCurrency::transfer(currency_id, from, to, amount),
			CurrencyId::Erc20(addr) => {
				if amount.is_zero() {
					return Ok(())
				}
				let asset = T::ConvertIntoAccountId::convert(addr);

				T::ContractAssets::transfer(asset, from.clone(), to.clone(), amount.into())
					.map(|_| ())
					.map_err(|err| err.error)
			},
		}
	}

	fn deposit(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		if amount.is_zero() {
			return Ok(())
		}
		match currency_id {
			CurrencyId::NativeToken(_) => T::MultiCurrency::deposit(currency_id, who, amount),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}

	fn withdraw(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		if amount.is_zero() {
			return Ok(())
		}
		match currency_id {
			CurrencyId::NativeToken(_) => T::MultiCurrency::withdraw(currency_id, who, amount),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}

	fn can_slash(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		value: Self::Balance,
	) -> bool {
		match currency_id {
			CurrencyId::NativeToken(_) => T::MultiCurrency::can_slash(currency_id, who, value),
			CurrencyId::Erc20(_) => value.is_zero(),
		}
	}

	fn slash(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> Self::Balance {
		match currency_id {
			CurrencyId::NativeToken(_) => T::MultiCurrency::slash(currency_id, who, amount),
			CurrencyId::Erc20(_) => Default::default(),
		}
	}
}

impl<T: Config> MultiCurrencyExtended<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		by_amount: Self::Amount,
	) -> sp_runtime::DispatchResult {
		match currency_id {
			CurrencyId::NativeToken(_) =>
				<T::MultiCurrency>::update_balance(currency_id, who, by_amount),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}
}

impl<T: Config> MultiLockableCurrency<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	type Moment = T::BlockNumber;

	fn set_lock(
		lock_id: orml_traits::LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		match currency_id {
			CurrencyId::NativeToken(_) =>
				<T::MultiCurrency>::set_lock(lock_id, currency_id, who, amount),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}

	fn extend_lock(
		lock_id: orml_traits::LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		match currency_id {
			CurrencyId::NativeToken(_) =>
				<T::MultiCurrency>::extend_lock(lock_id, currency_id, who, amount),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}

	fn remove_lock(
		lock_id: orml_traits::LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
	) -> sp_runtime::DispatchResult {
		match currency_id {
			CurrencyId::NativeToken(_) =>
				<T::MultiCurrency>::remove_lock(lock_id, currency_id, who),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}
}

impl<T: Config> MultiReservableCurrency<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	fn can_reserve(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		value: Self::Balance,
	) -> bool {
		match currency_id {
			CurrencyId::NativeToken(_) => <T::MultiCurrency>::can_reserve(currency_id, who, value),
			CurrencyId::Erc20(_) => false,
		}
	}

	fn slash_reserved(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		value: Self::Balance,
	) -> Self::Balance {
		match currency_id {
			CurrencyId::NativeToken(_) =>
				<T::MultiCurrency>::slash_reserved(currency_id, who, value),
			CurrencyId::Erc20(_) => Default::default(),
		}
	}

	fn reserved_balance(currency_id: Self::CurrencyId, who: &AccountIdOf<T>) -> Self::Balance {
		match currency_id {
			CurrencyId::NativeToken(_) => <T::MultiCurrency>::reserved_balance(currency_id, who),
			CurrencyId::Erc20(_) => Default::default(),
		}
	}

	fn reserve(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		value: Self::Balance,
	) -> sp_runtime::DispatchResult {
		match currency_id {
			CurrencyId::NativeToken(_) => <T::MultiCurrency>::reserve(currency_id, who, value),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}

	fn unreserve(
		currency_id: Self::CurrencyId,
		who: &AccountIdOf<T>,
		value: Self::Balance,
	) -> Self::Balance {
		match currency_id {
			CurrencyId::NativeToken(_) => <T::MultiCurrency>::unreserve(currency_id, who, value),
			CurrencyId::Erc20(_) => {
				log::debug!("not amount will be unreserved for contract based assets");
				Default::default()
			},
		}
	}

	fn repatriate_reserved(
		currency_id: Self::CurrencyId,
		slashed: &AccountIdOf<T>,
		beneficiary: &AccountIdOf<T>,
		value: Self::Balance,
		status: orml_traits::BalanceStatus,
	) -> core::result::Result<Self::Balance, DispatchError> {
		match currency_id {
			CurrencyId::NativeToken(_) => <T::MultiCurrency>::repatriate_reserved(
				currency_id,
				slashed,
				beneficiary,
				value,
				status,
			),
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
		}
	}
}

impl<T: Config> fungibles::Inspect<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	type AssetId = CurrencyId;

	type Balance = BalanceOf<T>;

	fn total_issuance(asset: Self::AssetId) -> Self::Balance {
		match asset {
			CurrencyId::Erc20(_) => <Self as MultiCurrency<AccountIdOf<T>>>::total_issuance(asset),
			_ => <T::MultiCurrency as fungibles::Inspect<AccountIdOf<T>>>::total_issuance(asset),
		}
	}

	fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
		match asset {
			CurrencyId::Erc20(_) => <Self as MultiCurrency<AccountIdOf<T>>>::minimum_balance(asset),
			_ => <T::MultiCurrency as fungibles::Inspect<AccountIdOf<T>>>::minimum_balance(asset),
		}
	}

	fn balance(asset: Self::AssetId, who: &AccountIdOf<T>) -> Self::Balance {
		match asset {
			CurrencyId::Erc20(_) =>
				<Self as MultiCurrency<AccountIdOf<T>>>::total_balance(asset, who),
			_ => <T::MultiCurrency as fungibles::Inspect<AccountIdOf<T>>>::balance(asset, who),
		}
	}

	fn reducible_balance(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		keep_alive: bool,
	) -> Self::Balance {
		match asset {
			CurrencyId::Erc20(_) =>
				<Self as MultiCurrency<AccountIdOf<T>>>::free_balance(asset, who),
			_ => <T::MultiCurrency as fungibles::Inspect<AccountIdOf<T>>>::reducible_balance(
				asset, who, keep_alive,
			),
		}
	}

	fn can_deposit(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
		mint: bool,
	) -> frame_support::traits::tokens::DepositConsequence {
		match asset {
			CurrencyId::Erc20(_) => {
				if amount.is_zero() {
					return DepositConsequence::Success
				}

				if <Self as fungibles::Inspect<_>>::total_issuance(asset)
					.checked_add(&amount)
					.is_none()
				{
					return DepositConsequence::Overflow
				}

				if <Self as fungibles::Inspect<_>>::balance(asset, who).saturating_add(amount) <
					<Self as fungibles::Inspect<_>>::minimum_balance(asset)
				{
					return DepositConsequence::BelowMinimum
				}

				DepositConsequence::Success
			},

			_ => <T::MultiCurrency as fungibles::Inspect<AccountIdOf<T>>>::can_deposit(
				asset, who, amount, mint,
			),
		}
	}

	fn can_withdraw(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::traits::tokens::WithdrawConsequence<Self::Balance> {
		match asset {
			CurrencyId::Erc20(_) =>
				match <Self as MultiCurrency<_>>::ensure_can_withdraw(asset, who, amount) {
					Ok(()) => WithdrawConsequence::Success,
					_ => WithdrawConsequence::NoFunds,
				},

			_ => <T::MultiCurrency as fungibles::Inspect<_>>::can_withdraw(asset, who, amount),
		}
	}
}

impl<T: Config> fungibles::Mutate<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	fn mint_into(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> DispatchResult {
		match asset {
			CurrencyId::Erc20(_) => <Self as MultiCurrency<_>>::deposit(asset, who, amount),
			_ => <T::MultiCurrency as MultiCurrency<_>>::deposit(asset, who, amount),
		}
	}

	fn burn_from(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		match asset {
			CurrencyId::Erc20(_) =>
				<Self as MultiCurrency<_>>::withdraw(asset, who, amount).map(|_| amount),
			_ => <T::MultiCurrency as fungibles::Mutate<_>>::burn_from(asset, who, amount),
		}
	}
}

impl<T: Config> fungibles::Transfer<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	fn transfer(
		asset: Self::AssetId,
		source: &AccountIdOf<T>,
		dest: &AccountIdOf<T>,
		amount: Self::Balance,
		keep_alive: bool,
	) -> Result<Self::Balance, DispatchError> {
		if amount.is_zero() {
			return Ok(Default::default())
		}

		match asset {
			CurrencyId::Erc20(_) =>
				<Self as MultiCurrency<_>>::transfer(asset, source, dest, amount).map(|_| amount),
			_ => <T::MultiCurrency as fungibles::Transfer<_>>::transfer(
				asset, source, dest, amount, keep_alive,
			),
		}
	}
}

impl<T: Config> fungibles::InspectHold<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	fn balance_on_hold(asset: Self::AssetId, who: &AccountIdOf<T>) -> Self::Balance {
		match asset {
			CurrencyId::Erc20(_) =>
				<Self as MultiReservableCurrency<AccountIdOf<T>>>::reserved_balance(asset, who),

			_ => <T::MultiCurrency as fungibles::InspectHold<_>>::balance_on_hold(asset, who),
		}
	}

	fn can_hold(asset: Self::AssetId, who: &AccountIdOf<T>, amount: Self::Balance) -> bool {
		match asset {
			CurrencyId::Erc20(_) =>
				<Self as MultiReservableCurrency<_>>::can_reserve(asset, who, amount),

			_ => <T::MultiCurrency as fungibles::InspectHold<_>>::can_hold(asset, who, amount),
		}
	}
}

impl<T: Config> fungibles::MutateHold<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	fn hold(asset: Self::AssetId, who: &AccountIdOf<T>, amount: Self::Balance) -> DispatchResult {
		match asset {
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
			_ => <T::MultiCurrency as fungibles::MutateHold<_>>::hold(asset, who, amount),
		}
	}

	fn release(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
		best_effort: bool,
	) -> Result<Self::Balance, DispatchError> {
		match asset {
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
			_ => <T::MultiCurrency as fungibles::MutateHold<_>>::release(
				asset,
				who,
				amount,
				best_effort,
			),
		}
	}

	fn transfer_held(
		asset: Self::AssetId,
		source: &AccountIdOf<T>,
		dest: &AccountIdOf<T>,
		amount: Self::Balance,
		best_effort: bool,
		on_hold: bool,
	) -> Result<Self::Balance, DispatchError> {
		match asset {
			CurrencyId::Erc20(_) => Err(Error::<T>::InvalidContractOperation.into()),
			_ => <T::MultiCurrency as fungibles::MutateHold<_>>::transfer_held(
				asset,
				source,
				dest,
				amount,
				best_effort,
				on_hold,
			),
		}
	}
}

impl<T: Config> fungibles::Unbalanced<AccountIdOf<T>> for Pallet<T>
where
	U256: From<BalanceOf<T>>,
{
	fn set_balance(
		asset: Self::AssetId,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> DispatchResult {
		match asset {
			CurrencyId::Erc20(_) => {
				log::warn!("setting balance for contract based asset is not allowed");
				Err(Error::<T>::InvalidContractOperation.into())
			},
			_ => <T::MultiCurrency as fungibles::Unbalanced<_>>::set_balance(asset, who, amount),
		}
	}

	fn set_total_issuance(asset: Self::AssetId, amount: Self::Balance) {
		match asset {
			CurrencyId::Erc20(_) => {
				log::warn!("no action will be executed for contract based assets");
			},
			_ => <T::MultiCurrency as fungibles::Unbalanced<_>>::set_total_issuance(asset, amount),
		}
	}
}
