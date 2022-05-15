use core::marker::PhantomData;

use crate::{AccountIdOf, AmountOf, BalanceOf, Config, Pallet};
use frame_support::traits::{
	fungible::{self, Inspect},
	fungibles,
	tokens::Balance,
	Currency as CurrencyT, Get,
};
use orml_traits::{
	BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
	MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
};
use primitives::CurrencyId;
use sp_core::U256;

/// single asset adapter that reuse T::MultiCurrency and the associated CurrencyId
pub struct CurrencyAdapter<T, GetCurrencyId>(PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> BasicCurrency<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn minimum_balance() -> Self::Balance {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::minimum_balance(GetCurrencyId::get())
	}

	fn total_issuance() -> Self::Balance {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::total_issuance(GetCurrencyId::get())
	}

	fn total_balance(who: &AccountIdOf<T>) -> Self::Balance {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::total_balance(GetCurrencyId::get(), who)
	}

	fn free_balance(who: &AccountIdOf<T>) -> Self::Balance {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::free_balance(GetCurrencyId::get(), who)
	}

	fn ensure_can_withdraw(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::ensure_can_withdraw(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn transfer(
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::transfer(
			GetCurrencyId::get(),
			from,
			to,
			amount,
		)
	}

	fn deposit(who: &AccountIdOf<T>, amount: Self::Balance) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::deposit(GetCurrencyId::get(), who, amount)
	}

	fn withdraw(who: &AccountIdOf<T>, amount: Self::Balance) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::deposit(GetCurrencyId::get(), who, amount)
	}

	fn can_slash(who: &AccountIdOf<T>, value: Self::Balance) -> bool {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::can_slash(GetCurrencyId::get(), who, value)
	}

	fn slash(who: &AccountIdOf<T>, amount: Self::Balance) -> Self::Balance {
		<Pallet<T> as MultiCurrency<AccountIdOf<T>>>::slash(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> BasicCurrencyExtended<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(who: &AccountIdOf<T>, by_amount: Self::Amount) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiCurrencyExtended<AccountIdOf<T>>>::update_balance(
			GetCurrencyId::get(),
			who,
			by_amount,
		)
	}
}

impl<T, GetCurrencyId> BasicLockableCurrency<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Moment = T::BlockNumber;

	fn set_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiLockableCurrency<AccountIdOf<T>>>::set_lock(
			lock_id,
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn extend_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiLockableCurrency<AccountIdOf<T>>>::extend_lock(
			lock_id,
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn remove_lock(
		lock_id: orml_traits::LockIdentifier,
		who: &AccountIdOf<T>,
	) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiLockableCurrency<AccountIdOf<T>>>::remove_lock(
			lock_id,
			GetCurrencyId::get(),
			who,
		)
	}
}

impl<T, GetCurrencyId> BasicReservableCurrency<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn can_reserve(who: &AccountIdOf<T>, value: Self::Balance) -> bool {
		<Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::can_reserve(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn slash_reserved(who: &AccountIdOf<T>, value: Self::Balance) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::slash_reserved(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn reserved_balance(who: &AccountIdOf<T>) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::reserved_balance(
			GetCurrencyId::get(),
			who,
		)
	}

	fn reserve(who: &AccountIdOf<T>, value: Self::Balance) -> sp_runtime::DispatchResult {
		<Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::reserve(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn unreserve(who: &AccountIdOf<T>, value: Self::Balance) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::unreserve(
			GetCurrencyId::get(),
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
		<Pallet<T> as MultiReservableCurrency<AccountIdOf<T>>>::repatriate_reserved(
			GetCurrencyId::get(),
			slashed,
			beneficiary,
			value,
			status,
		)
	}
}

impl<T, GetCurrencyId> fungible::Inspect<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn total_issuance() -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::total_issuance(GetCurrencyId::get())
	}

	fn minimum_balance() -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::minimum_balance(GetCurrencyId::get())
	}

	fn balance(who: &AccountIdOf<T>) -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::balance(GetCurrencyId::get(), who)
	}

	fn reducible_balance(who: &AccountIdOf<T>, keep_alive: bool) -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::reducible_balance(
			GetCurrencyId::get(),
			who,
			keep_alive,
		)
	}

	fn can_deposit(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::traits::tokens::DepositConsequence {
		<Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::can_deposit(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn can_withdraw(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::traits::tokens::WithdrawConsequence<Self::Balance> {
		<Pallet<T> as fungibles::Inspect<AccountIdOf<T>>>::can_withdraw(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}
}

impl<T, GetCurrencyId> fungible::Mutate<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn mint_into(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::dispatch::DispatchResult {
		<Pallet<T> as fungibles::Mutate<AccountIdOf<T>>>::mint_into(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn burn_from(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<Pallet<T> as fungibles::Mutate<AccountIdOf<T>>>::burn_from(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}
}

impl<T, GetCurrencyId> fungible::InspectHold<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn balance_on_hold(who: &AccountIdOf<T>) -> Self::Balance {
		<Pallet<T> as fungibles::InspectHold<AccountIdOf<T>>>::balance_on_hold(
			GetCurrencyId::get(),
			who,
		)
	}

	fn can_hold(who: &AccountIdOf<T>, amount: Self::Balance) -> bool {
		<Pallet<T> as fungibles::InspectHold<AccountIdOf<T>>>::can_hold(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}
}

impl<T, GetCurrencyId> fungible::Transfer<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn transfer(
		source: &AccountIdOf<T>,
		dest: &AccountIdOf<T>,
		amount: Self::Balance,
		keep_alive: bool,
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<Pallet<T> as fungibles::Transfer<AccountIdOf<T>>>::transfer(
			GetCurrencyId::get(),
			source,
			dest,
			amount,
			keep_alive,
		)
	}
}

impl<T, GetCurrencyId> fungible::MutateHold<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn hold(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::dispatch::DispatchResult {
		<Pallet<T> as fungibles::MutateHold<AccountIdOf<T>>>::hold(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn release(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
		best_effort: bool,
	) -> Result<Self::Balance, sp_runtime::DispatchError> {
		<Pallet<T> as fungibles::MutateHold<AccountIdOf<T>>>::release(
			GetCurrencyId::get(),
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
		<Pallet<T> as fungibles::MutateHold<AccountIdOf<T>>>::transfer_held(
			GetCurrencyId::get(),
			source,
			dest,
			amount,
			best_effort,
			on_held,
		)
	}
}

impl<T, GetCurrencyId> fungible::Unbalanced<AccountIdOf<T>> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyId>,
	U256: From<BalanceOf<T>>,
{
	fn set_balance(
		who: &AccountIdOf<T>,
		amount: Self::Balance,
	) -> frame_support::dispatch::DispatchResult {
		<Pallet<T> as fungibles::Unbalanced<AccountIdOf<T>>>::set_balance(
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn set_total_issuance(amount: Self::Balance) {
		<Pallet<T> as fungibles::Unbalanced<AccountIdOf<T>>>::set_total_issuance(
			GetCurrencyId::get(),
			amount,
		)
	}
}

// adapter where
pub struct BasicCurrencyAdapter;
