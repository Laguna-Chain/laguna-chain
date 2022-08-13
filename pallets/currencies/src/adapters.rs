//! # single currency adapter
//!
//! For scenario where only one token is considered, we can use the adapter created here to expose
//! the currency module as a single currency system.
//!
//! The adapter accepts a generic type parameter `AssetIdGeter` where consumer  can specify the only
//! needed asset specifier.

use core::marker::PhantomData;

use frame_support::{
	sp_runtime,
	traits::{fungible, fungibles, Get},
};
use orml_traits::{
	BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
	MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
};
use primitives::CurrencyId;
use sp_core::U256;

use crate::{AccountIdOf, AmountOf, BalanceOf};

pub struct CurrencyAdapter<T, AssetIdGetter>(PhantomData<(T, AssetIdGetter)>);

impl<T, AssetIdGetter> fungible::Inspect<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn total_issuance() -> Self::Balance {
		<crate::Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::total_issuance(
			AssetIdGetter::get(),
		)
	}

	fn minimum_balance() -> Self::Balance {
		<crate::Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::minimum_balance(
			AssetIdGetter::get(),
		)
	}

	fn balance(who: &AccountIdOf<T>) -> Self::Balance {
		<crate::Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::balance(AssetIdGetter::get(), who)
	}

	fn reducible_balance(who: &AccountIdOf<T>, keep_alive: bool) -> Self::Balance {
		<crate::Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::reducible_balance(
			AssetIdGetter::get(),
			who,
			keep_alive,
		)
	}

	fn can_deposit(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
		mint: bool,
	) -> frame_support::traits::tokens::DepositConsequence {
		<crate::Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::can_deposit(
			AssetIdGetter::get(),
			who,
			amount,
			mint,
		)
	}

	fn can_withdraw(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::traits::tokens::WithdrawConsequence<Self::Balance> {
		<crate::Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::can_withdraw(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}
}

impl<T, AssetIdGetter> fungible::Mutate<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn mint_into(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::dispatch::DispatchResult {
		<crate::Pallet<T> as fungibles::Mutate<AccountIdOf<T>>>::mint_into(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn burn_from(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<crate::Pallet<T> as fungibles::Mutate<AccountIdOf<T>>>::burn_from(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}
}

impl<T, AssetIdGetter> fungible::Transfer<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn transfer(
		source: &AccountIdOf<T>,
		dest: &AccountIdOf<T>,
		amount: Self::Balance,
		keep_alive: bool,
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<crate::Pallet<T> as fungibles::Transfer<AccountIdOf<T>>>::transfer(
			AssetIdGetter::get(),
			source,
			dest,
			amount,
			keep_alive,
		)
	}
}

impl<T, AssetIdGetter> fungible::Unbalanced<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn set_balance(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::dispatch::DispatchResult {
		<crate::Pallet<T> as fungibles::Unbalanced<AccountIdOf<T>>>::set_balance(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn set_total_issuance(amount: Self::Balance) {
		<crate::Pallet<T> as fungibles::Unbalanced<AccountIdOf<T>>>::set_total_issuance(
			AssetIdGetter::get(),
			amount,
		)
	}
}

impl<T, AssetIdGetter> fungible::InspectHold<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn balance_on_hold(who: &AccountIdOf<T>) -> Self::Balance {
		<crate::Pallet<T> as fungibles::InspectHold<AccountIdOf<T>>>::balance_on_hold(
			AssetIdGetter::get(),
			who,
		)
	}

	fn can_hold(who: &AccountIdOf<T>, amount: Self::Balance) -> bool {
		<crate::Pallet<T> as fungibles::InspectHold<AccountIdOf<T>>>::can_hold(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}
}

impl<T, AssetIdGetter> fungible::MutateHold<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn hold(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::dispatch::DispatchResult {
		<crate::Pallet<T> as fungibles::MutateHold<AccountIdOf<T>>>::hold(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn release(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
		best_effort: bool,
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<crate::Pallet<T> as fungibles::MutateHold<AccountIdOf<T>>>::release(
			AssetIdGetter::get(),
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
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<crate::Pallet<T> as fungibles::MutateHold<AccountIdOf<T>>>::transfer_held(
			AssetIdGetter::get(),
			source,
			dest,
			amount,
			best_effort,
			on_held,
		)
	}
}

impl<T, AssetIdGetter> BasicCurrency<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn minimum_balance() -> Self::Balance {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::minimum_balance(AssetIdGetter::get())
	}

	fn total_issuance() -> Self::Balance {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::total_issuance(AssetIdGetter::get())
	}

	fn total_balance(who: &AccountIdOf<T>) -> Self::Balance {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::total_balance(
			AssetIdGetter::get(),
			who,
		)
	}

	fn free_balance(who: &AccountIdOf<T>) -> Self::Balance {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::free_balance(AssetIdGetter::get(), who)
	}

	fn ensure_can_withdraw(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::ensure_can_withdraw(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn transfer(
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::transfer(
			AssetIdGetter::get(),
			from,
			to,
			amount,
		)
	}

	fn deposit(who: &AccountIdOf<T>, amount: Self::Balance) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::deposit(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn withdraw(who: &AccountIdOf<T>, amount: Self::Balance) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::withdraw(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn can_slash(who: &AccountIdOf<T>, value: Self::Balance) -> bool {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::can_slash(
			AssetIdGetter::get(),
			who,
			value,
		)
	}

	fn slash(who: &AccountIdOf<T>, amount: Self::Balance) -> Self::Balance {
		<crate::Pallet<T> as MultiCurrency<AccountIdOf<T>>>::slash(
			AssetIdGetter::get(),
			who,
			amount,
		)
	}
}

impl<T, AssetIdGetter> BasicCurrencyExtended<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(who: &AccountIdOf<T>, by_amount: Self::Amount) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiCurrencyExtended<AccountIdOf<T>>>::update_balance(
			AssetIdGetter::get(),
			who,
			by_amount,
		)
	}
}

impl<T, AssetIdGetter> BasicLockableCurrency<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Moment = <crate::Pallet<T> as BasicLockableCurrency<AccountIdOf<T>>>::Moment;

	fn set_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiLockableCurrency<AccountIdOf<T>>>::set_lock(
			lock_id,
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn extend_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiLockableCurrency<AccountIdOf<T>>>::extend_lock(
			lock_id,
			AssetIdGetter::get(),
			who,
			amount,
		)
	}

	fn remove_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
	) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiLockableCurrency<AccountIdOf<T>>>::remove_lock(
			lock_id,
			AssetIdGetter::get(),
			who,
		)
	}
}

impl<T, AssetIdGetter> BasicReservableCurrency<AccountIdOf<T>> for CurrencyAdapter<T, AssetIdGetter>
where
	T: crate::Config,
	AssetIdGetter: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn can_reserve(who: &AccountIdOf<T>, value: Self::Balance) -> bool {
		<crate::Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::can_reserve(
			AssetIdGetter::get(),
			who,
			value,
		)
	}

	fn slash_reserved(who: &AccountIdOf<T>, value: Self::Balance) -> Self::Balance {
		<crate::Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::slash_reserved(
			AssetIdGetter::get(),
			who,
			value,
		)
	}

	fn reserved_balance(who: &AccountIdOf<T>) -> Self::Balance {
		<crate::Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::reserved_balance(
			AssetIdGetter::get(),
			who,
		)
	}

	fn reserve(who: &AccountIdOf<T>, value: Self::Balance) -> sp_runtime::DispatchResult {
		<crate::Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::reserve(
			AssetIdGetter::get(),
			who,
			value,
		)
	}

	fn unreserve(who: &AccountIdOf<T>, value: Self::Balance) -> Self::Balance {
		<crate::Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::unreserve(
			AssetIdGetter::get(),
			who,
			value,
		)
	}

	fn repatriate_reserved(
		slashed: &AccountIdOf<T>,
		beneficiary: &AccountIdOf<T>,
		value: Self::Balance,
		status: orml_traits::BalanceStatus,
	) -> core::result::Result<Self::Balance, sp_runtime::DispatchError> {
		<crate::Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::repatriate_reserved(
			AssetIdGetter::get(),
			slashed,
			beneficiary,
			value,
			status,
		)
	}
}
