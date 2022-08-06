//# # fluent fee
//#
//# this modules customize and replace the how fee is charged for a given transaction

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::Dispatchable, pallet_prelude::*, sp_runtime::traits::Saturating,
	traits::WithdrawReasons,
};

use frame_system::pallet_prelude::*;
use scale_info::prelude::boxed::Box;

use orml_traits::{arithmetic::Zero, MultiCurrency};
use primitives::CurrencyId;

use frame_support::sp_runtime::traits::{DispatchInfoOf, PostDispatchInfoOf};
use pallet_transaction_payment::OnChargeTransaction;
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource, IsFeeSharingCall};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::weights::GetDispatchInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type DefaultFeeAsset: Get<CurrencyId>;
		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		type Call: Parameter
			+ Dispatchable<Origin = <Self as frame_system::Config>::Origin>
			+ From<frame_system::Call<Self>>
			+ GetDispatchInfo;

		type IsFeeSharingCall: IsFeeSharingCall<Self, AccountId = AccountIdOf<Self>>;

		type FeeSource: FeeSource<AccountId = AccountIdOf<Self>, AssetId = CurrencyId>;
		type FeeMeasure: FeeMeasure<AssetId = CurrencyId, Balance = BalanceOf<Self>>;
		type FeeDispatch: FeeDispatch<Self, AssetId = CurrencyId, Balance = BalanceOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		AccountPreferenceUpdated { account: AccountIdOf<T>, currency: Option<CurrencyId> },
		FeeSharingBeneficiaryIncluded { beneficiary: Option<AccountIdOf<T>> },
		FeeSharedWithTheBeneficiary { beneficiary: AccountIdOf<T>, amount: BalanceOf<T> },
		FeeWithdrawn { currency: CurrencyId, amount: BalanceOf<T> },
		FeeRefunded { currency: CurrencyId, amount: BalanceOf<T> },
		FeePayout { receiver: AccountIdOf<T>, currency: CurrencyId, amount: BalanceOf<T> },
		FeeCorrected,
	}

	#[pallet::storage]
	pub(super) type DefdaultFeeSource<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, CurrencyId>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// set the default asset for this account
		#[pallet::weight(1000)]
		pub fn set_default(origin: OriginFor<T>, asset_id: CurrencyId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			DefdaultFeeSource::<T>::insert(who.clone(), asset_id);
			Self::deposit_event(Event::AccountPreferenceUpdated {
				account: who,
				currency: Some(asset_id),
			});
			Ok(())
		}

		/// unset the default asset for this account
		#[pallet::weight(1000)]
		pub fn unset_default(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			DefdaultFeeSource::<T>::remove(who.clone());
			Self::deposit_event(Event::AccountPreferenceUpdated { account: who, currency: None });

			Ok(())
		}

		// dynamically add one unit of weight if beneficiary is_some
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(
				dispatch_info.weight,
				dispatch_info.class,
				dispatch_info.pays_fee,
			)
		})]
		pub fn fee_sharing_wrapper(
			origin: OriginFor<T>,
			call: Box<<T as pallet::Config>::Call>,
			beneficiary: Option<AccountIdOf<T>>,
		) -> DispatchResult {
			ensure_signed(origin.clone())?;
			if beneficiary.is_some() {
				Self::deposit_event(Event::FeeSharingBeneficiaryIncluded { beneficiary });
			}
			match call.dispatch(origin) {
				Ok(_) => Ok(()),
				Err(_) => Err(DispatchError::Other("Fee sharing type dispatch failed")),
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_fee_source_priority(
		account: &<T as frame_system::Config>::AccountId,
	) -> Option<<T::FeeSource as FeeSource>::AssetId> {
		DefdaultFeeSource::<T>::get(account)
	}
}

type CallOf<T> = <T as frame_system::Config>::Call;

// overview of stages during a multi-assets payout
//
// 1. gather the weight for a call
// 2. determine the specified asset is a legal asset to paid as fee
// 3. determine the conversion ratio between target asset and native token
// 4. withdraw comparable amount target toekn of native token
// 5. pass already withdrawn to next stage for correction and payout
// 6. split tip amount and fee amount
// 7. tip the block author and manipulate the native asets accordingly
// 8. compute over withdrawn amount from actual fee and withdrawn
// 9. return unused target token back to the account

/// record multicurrency payout info
pub struct MultiCurrencyPayout<T: Config> {
	// asset_id user requested to pay as fee
	source_asset_id: CurrencyId,
	// native amount needed
	request_amount_native: BalanceOf<T>,
	// equivalent withdrawn
	withdrawn_source_amount: BalanceOf<T>,
	beneficiary: Option<AccountIdOf<T>>,
}

impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config,
	T: pallet_transaction_payment::Config,
{
	type Balance = BalanceOf<T>;

	// TODO: deal with correct liquidity info logic
	type LiquidityInfo = Option<MultiCurrencyPayout<T>>;

	fn withdraw_fee(
		who: &T::AccountId,
		call: &<T as frame_system::Config>::Call,
		dispatch_info: &DispatchInfoOf<CallOf<T>>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		// no fees aquired
		if fee.is_zero() {
			return Ok(None)
		}

		let preferred_fee_asset =
			Self::account_fee_source_priority(who).unwrap_or_else(T::DefaultFeeAsset::get);

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

		let amount = T::FeeMeasure::measure(&preferred_fee_asset, fee)?;

		match T::FeeDispatch::withdraw(who, &preferred_fee_asset, call, &amount, &withdraw_reason) {
			Ok(_) => {
				let payout_info = MultiCurrencyPayout {
					source_asset_id: preferred_fee_asset,
					request_amount_native: fee,
					withdrawn_source_amount: amount,
					beneficiary: T::IsFeeSharingCall::is_call(call),
				};
				log::debug!(target: "fluent_fee::withdrawn", "succsefully withdrawn from {preferred_fee_asset:?}");

				Pallet::<T>::deposit_event(Event::<T>::FeeWithdrawn {
					currency: preferred_fee_asset,
					amount,
				});

				Ok(Some(payout_info))
			},
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &DispatchInfoOf<CallOf<T>>,
		post_info: &PostDispatchInfoOf<CallOf<T>>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), frame_support::unsigned::TransactionValidityError> {
		// TODO: execute refund plan from already_withdrawn

		if let Some(MultiCurrencyPayout {
			source_asset_id,
			request_amount_native,
			withdrawn_source_amount,
			beneficiary,
		}) = already_withdrawn
		{
			// overcharged amount in native
			let overcharged_amount_native = request_amount_native.saturating_sub(corrected_fee);

			let mut corrected_withdrawn = withdrawn_source_amount;

			if !overcharged_amount_native.is_zero() {
				log::debug!(target: "fluent_fee::correct", "succsefully withdrawn from {source_asset_id:?}");
				let amounts_source =
					T::FeeMeasure::measure(&source_asset_id, overcharged_amount_native)?;

				// it's possible refund failed, due to below E.D or routing temporary not possible
				if let Ok(refunded) = T::FeeDispatch::refund(who, &source_asset_id, &amounts_source)
				{
					corrected_withdrawn = withdrawn_source_amount.saturating_sub(refunded);
					Pallet::<T>::deposit_event(Event::<T>::FeeRefunded {
						currency: source_asset_id,
						amount: refunded,
					});
				}
			}

			// pay the tips for the block author
			let tip_amount_source = T::FeeMeasure::measure(&source_asset_id, tip)?;
			if T::FeeDispatch::tip(&source_asset_id, &tip_amount_source).is_ok() {
				corrected_withdrawn = corrected_withdrawn.saturating_sub(tip_amount_source);
			}

			// splits the remaining balances between all needed parties.
			T::FeeDispatch::post_info_correction(
				&source_asset_id,
				&corrected_withdrawn,
				&beneficiary,
			)
			.map_err(|_| {
				frame_support::unsigned::TransactionValidityError::Invalid(
					InvalidTransaction::Payment,
				)
			})?;

			Pallet::<T>::deposit_event(Event::<T>::FeeCorrected);
		}

		Ok(())
	}
}
