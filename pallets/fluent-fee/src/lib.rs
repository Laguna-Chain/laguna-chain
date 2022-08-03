//# # fluent fee
//#
//# this modules customize and replace the how fee is charged for a given transaction

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::WithdrawReasons, weights::{GetDispatchInfo, PostDispatchInfo}, dispatch::Dispatchable};
use frame_system::pallet_prelude::*;
use codec::{Encode, Decode};
use orml_traits::{arithmetic::Zero, MultiCurrency};
use primitives::{CurrencyId, TokenId};

pub use pallet::*;
use pallet_transaction_payment::OnChargeTransaction;
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod signed_extension;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type DefaultFeeAsset: Get<CurrencyId>;
		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;
		type Call: Parameter + Dispatchable<Origin = <Self as frame_system::Config>::Origin> + From<frame_system::Call<Self>>;

		type FeeSource: FeeSource<AccountId = AccountIdOf<Self>, AssetId = CurrencyId>;
		type FeeMeasure: FeeMeasure<AssetId = CurrencyId, Balance = BalanceOf<Self>>;
		type FeeDispatch: FeeDispatch<Self, AssetId = CurrencyId, Balance = BalanceOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AccountPreferenceUpdated((AccountIdOf<T>, Option<CurrencyId>)),
	}

	#[pallet::storage]
	pub(super) type DefaultFeeSource<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, CurrencyId>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		pub fn set_default(origin: OriginFor<T>, asset_id: CurrencyId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			DefaultFeeSource::<T>::insert(who.clone(), asset_id);
			Self::deposit_event(Event::AccountPreferenceUpdated((who, Some(asset_id))));
			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn unset_default(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			DefaultFeeSource::<T>::remove(who.clone());
			Self::deposit_event(Event::AccountPreferenceUpdated((who, None)));

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn fee_sharing_wrapper(
			origin: OriginFor<T>,
			call:  Box<<T as pallet::Config>::Call>,
			beneficiary: Option<AccountIdOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			// let dispatch_origin = origin.clone().into();
			match call.dispatch(origin) {
				Ok(_) => Ok(()),
				Err(_) => Err(DispatchError::Other("Scheduled call dispatch error")),
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_fee_source_priority(
		account: &<T as frame_system::Config>::AccountId,
	) -> Option<<T::FeeSource as FeeSource>::AssetId> {
		DefaultFeeSource::<T>::get(account)
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
		dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<<T as frame_system::Config>::Call>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		// no fees aquired
		if fee.is_zero() {
			return Ok(())
		}

		let preferred_fee_asset =
			Self::account_fee_source_priority(who).unwrap_or_else(|| T::DefaultFeeAsset::get());

		// check if preferenced fee source is both listed and accepted
		T::FeeSource::listed(&preferred_fee_asset)
			.and_then(|_| T::FeeSource::accepted(who, &preferred_fee_asset))
			.map_err(|e| {
				log::debug!("{:?}", e);
				TransactionValidityError::from(InvalidTransaction::Payment)
			})?;

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		let amounts = T::FeeMeasure::measure(&preferred_fee_asset, fee)?;

		match T::FeeDispatch::withdraw(who, &preferred_fee_asset, call, &amounts, &withdraw_reason)
		{
			Ok(_) => {
				log::debug!(target: "fluent_fee::withdrawn", "succsefully withdrawn using native_currency");
				Ok(())
			},
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<<T as frame_system::Config>::Call>,
		post_info: &frame_support::sp_runtime::traits::PostDispatchInfoOf<<T as frame_system::Config>::Call>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), frame_support::unsigned::TransactionValidityError> {
		// TODO: execute refund plan from already_withdrawn

		log::debug!(target: "fluent_fee::post_deposit", "deposit without refund");

		let preferred_fee_asset =
			Self::account_fee_source_priority(who).unwrap_or_else(|| T::DefaultFeeAsset::get());

		match T::FeeDispatch::post_info_correction(&preferred_fee_asset, post_info) {
			Ok(_) => Ok(()),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}
}
