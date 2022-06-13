#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	traits::{Currency, WithdrawReasons},
};

use frame_system::{ensure_signed, pallet_prelude::OriginFor, WeightInfo};
use orml_traits::{
	arithmetic::{CheckedAdd, Zero},
	LockIdentifier, MultiCurrency, MultiLockableCurrency,
};
use primitives::{CurrencyId, TokenId};
use scale_info::TypeInfo;

pub use pallet::*;
use pallet_transaction_payment::OnChargeTransaction;
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource};

type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// # fluent fee
///
/// this modules customize and replace the how fee is charged for a given transaction
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Call: From<Call<Self>> + Dispatchable;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		type LockId: Get<LockIdentifier>;

		type MultiCurrency: MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		type FeeSource: FeeSource<AssetId = CurrencyId, Balance = BalanceOf<Self>>;
		type FeeMeasure: FeeMeasure<AssetId = CurrencyId, Balance = BalanceOf<Self>>;
		type FeeDispatch: FeeDispatch<Self, AssetId = CurrencyId, Balance = BalanceOf<Self>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn treasury_balance_per_account)]
	pub type TreasuryBalancePerAccount<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_staked)]
	pub type TotalStaked<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn accepted_assets)]
	pub type AcceptedAssets<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn account_fee_source_priority)]
	pub type AccountFeeSourcePriority<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, CurrencyId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TreasuryDeposit { account: T::AccountId, amount: BalanceOf<T> },
		TotalStakeUpdated { currency: CurrencyId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn prepay_fees(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			let current_prepaid_fee_amount = Self::treasury_balance_per_account(&from);

			T::MultiCurrency::transfer(currency_id, &from, &T::TreasuryAccount::get(), amount)?;

			let updated_prepaid_fee_amount =
				current_prepaid_fee_amount.checked_add(&amount).unwrap();

			<TreasuryBalancePerAccount<T>>::insert(&from, updated_prepaid_fee_amount);

			// Self::deposit_event(Event::TreasuryDeposit {
			// 	account: from,
			// 	amount: updated_prepaid_fee_amount,
			// });

			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn set_fee_source_priority(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			T::FeeSource::accepted(&currency_id)
				.expect("Your preferred currency is not an accepted fee source");
			<AccountFeeSourcePriority<T>>::insert(&from, currency_id);

			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn stake(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			let total_staked = Self::total_staked(&currency_id);

			T::MultiCurrency::set_lock(T::LockId::get(), currency_id, &from, amount)?;

			let updated_total_stake = total_staked.checked_add(&amount).unwrap();

			<TotalStaked<T>>::insert(&currency_id, updated_total_stake);

			Self::deposit_event(Event::TotalStakeUpdated {
				currency: currency_id,
				amount: updated_total_stake,
			});

			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn listing_asset(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResult {
			let from = ensure_signed(origin)?;
			T::FeeSource::listing_asset(&currency_id)?;
			Ok(())
		}

		#[pallet::weight(1000_000)]
		pub fn check_accepted_asset(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			T::FeeSource::accepted(&currency_id)?;
			Ok(())
		}
	}
}

impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config,
	T: pallet_transaction_payment::Config,
{
	type Balance = BalanceOf<T>;

	// TODO: deal with correct liquidity info logic
	type LiquidityInfo = ();

	fn withdraw_fee(
		who: &T::AccountId,
		call: &<T as frame_system::Config>::Call,
		dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<
			<T as frame_system::Config>::Call,
		>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		// no fees aquired
		if fee.is_zero() {
			return Ok(())
		}

		let preferred_fee_asset = <AccountFeeSourcePriority<T>>::get(&who)
			.unwrap_or(CurrencyId::NativeToken(TokenId::Laguna));

		// check if preferenced fee source is accepted
		T::FeeSource::accepted(&preferred_fee_asset)
			.map_err(|e| TransactionValidityError::from(InvalidTransaction::Payment))?;

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		let amounts = T::FeeMeasure::measure(&preferred_fee_asset, fee)?;

		match T::FeeDispatch::withdraw(who, &preferred_fee_asset, &amounts, &fee, &withdraw_reason)
		{
			Ok(_) => {
				log::info!(target: "fee withdrawn", "successfully withdrawn using native_currency");
				Ok(())
			},
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<
			<T as frame_system::Config>::Call,
		>,
		post_info: &frame_support::sp_runtime::traits::PostDispatchInfoOf<
			<T as frame_system::Config>::Call,
		>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), frame_support::unsigned::TransactionValidityError> {
		// TODO: execute refund plan from already_withdrawn

		log::info!(target: "fee correction", "deposit without refund");

		let preferred_fee_asset = <AccountFeeSourcePriority<T>>::get(&who)
			.unwrap_or(CurrencyId::NativeToken(TokenId::Laguna));

		match T::FeeDispatch::post_info_correction(&preferred_fee_asset, post_info) {
			Ok(_) => Ok(()),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}
}
