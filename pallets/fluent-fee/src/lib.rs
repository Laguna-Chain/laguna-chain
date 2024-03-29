//# # fluent fee
//#
//# this modules customize and replace the how fee is charged for a given transaction

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	sp_runtime::{
		sp_std::prelude::*,
		traits::{AccountIdConversion, Saturating},
	},
	traits::WithdrawReasons,
};

use frame_system::pallet_prelude::*;

use orml_traits::{arithmetic::Zero, MultiCurrency};

use frame_support::sp_runtime::traits::{DispatchInfoOf, PostDispatchInfoOf};
use pallet_transaction_payment::OnChargeTransaction;
use traits::fee::{CallFilterWithOutput, FeeCarrier, FeeDispatch, FeeMeasure, FeeSource};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T, C> = <C as MultiCurrency<AccountIdOf<T>>>::Balance;
type CallOf<T> = <T as frame_system::Config>::Call;
type CurrencyOf<T, C> = <C as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::{sp_runtime::FixedPointNumber, weights::GetDispatchInfo, PalletId};
	use traits::fee::{CallFilterWithOutput, FeeCarrier};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		// set a global default for fee preference
		type DefaultFeeAsset: Get<CurrencyOf<Self, Self::MultiCurrency>>;

		// monetary system to be used
		type MultiCurrency: MultiCurrency<Self::AccountId>;

		// call wrapping
		type Call: Parameter
			+ Dispatchable<Origin = <Self as frame_system::Config>::Origin>
			+ From<frame_system::Call<Self>>
			+ GetDispatchInfo;

		// call_filter for shared call
		type IsFeeSharingCall: CallFilterWithOutput<
			Call = CallOf<Self>,
			Output = Option<(AccountIdOf<Self>, BalanceOf<Self, Self::MultiCurrency>)>,
		>;

		type IsCarrierAttachedCall: CallFilterWithOutput<
			Call = CallOf<Self>,
			Output = Option<(
				AccountIdOf<Self>,
				Vec<u8>,
				BalanceOf<Self, Self::MultiCurrency>,
				Weight,
				Option<BalanceOf<Self, Self::MultiCurrency>>,
				bool,
			)>,
		>;
		// fee source evaluation
		type FeeSource: FeeSource<
			AccountId = AccountIdOf<Self>,
			AssetId = CurrencyOf<Self, Self::MultiCurrency>,
		>;

		// fee rate evaluation
		type FeeMeasure: FeeMeasure<
			AssetId = CurrencyOf<Self, Self::MultiCurrency>,
			Balance = BalanceOf<Self, Self::MultiCurrency>,
		>;

		// withdraw and redeem path
		type FeeDispatch: FeeDispatch<
			AccountId = AccountIdOf<Self>,
			AssetId = CurrencyOf<Self, Self::MultiCurrency>,
			Balance = BalanceOf<Self, Self::MultiCurrency>,
		>;

		type Ratio: FixedPointNumber;

		/// treasury | block_author | beneficiary
		type PayoutSplits: Get<(Self::Ratio, Self::Ratio)>;

		/// pallet to collect native token from carrier
		type PalletId: Get<PalletId>;

		/// carrier executer
		type Carrier: FeeCarrier<
			AccountId = AccountIdOf<Self>,
			Balance = BalanceOf<Self, Self::MultiCurrency>,
		>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		AccountPreferenceUpdated {
			account: AccountIdOf<T>,
			currency: Option<CurrencyOf<T, T::MultiCurrency>>,
		},
		FeeWithdrawn {
			currency: CurrencyOf<T, T::MultiCurrency>,
			amount: BalanceOf<T, T::MultiCurrency>,
		},
		FeeRefunded {
			currency: CurrencyOf<T, T::MultiCurrency>,
			amount: BalanceOf<T, T::MultiCurrency>,
		},
		FallbackToNative,
		FeePayout {
			receiver: AccountIdOf<T>,
			currency: CurrencyOf<T, T::MultiCurrency>,
			amount: BalanceOf<T, T::MultiCurrency>,
		},
		FeeCorrected,
		CarrierAttached {
			carrier_address: AccountIdOf<T>,
			carrier_data: Vec<u8>,
			post_transfer: bool,
		},
		CarrierExecute {
			carrier_address: AccountIdOf<T>,
			carrier_data: Vec<u8>,
			post_transfer: bool,
		},
		ValueAddedFeeSpecified {
			recipient: AccountIdOf<T>,
			balance: BalanceOf<T, T::MultiCurrency>,
		},
		ValueAddedFeeHonored {
			recipient: AccountIdOf<T>,
			balance: BalanceOf<T, T::MultiCurrency>,
		},
	}

	#[pallet::storage]
	pub(super) type DefdaultFeeSource<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, CurrencyOf<T, T::MultiCurrency>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// set the default asset for this account
		#[pallet::weight(1000)]
		pub fn set_default(
			origin: OriginFor<T>,
			asset_id: CurrencyOf<T, T::MultiCurrency>,
		) -> DispatchResult {
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

		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(
				dispatch_info.weight,
				dispatch_info.class,
				dispatch_info.pays_fee,
			)
		})]
		pub fn fluent_fee_wrapper(
			origin: OriginFor<T>,
			call: Box<<T as pallet::Config>::Call>, // used to get the weight
			carrier_info: Option<(
				AccountIdOf<T>,
				Vec<u8>,
				BalanceOf<T, T::MultiCurrency>,
				Weight,
				Option<BalanceOf<T, T::MultiCurrency>>,
				bool,
			)>,
			value_added_info: Option<(AccountIdOf<T>, BalanceOf<T, T::MultiCurrency>)>,
		) -> DispatchResult {
			ensure_signed(origin.clone())?;

			// TODO: we might want to create condition to allow the inclusion of the carrier
			if let Some((carrier_address, carrier_data, .., post_transfer)) = carrier_info {
				Self::deposit_event(Event::<T>::CarrierAttached {
					carrier_address,
					carrier_data,
					post_transfer,
				});
			}

			if let Some((recipient, balance)) = value_added_info {
				Self::deposit_event(Event::<T>::ValueAddedFeeSpecified { recipient, balance });
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
	source_asset_id: CurrencyOf<T, T::MultiCurrency>,
	// native amount needed
	request_amount_native: BalanceOf<T, T::MultiCurrency>,
	// equivalent withdrawn
	withdrawn_source_amount: BalanceOf<T, T::MultiCurrency>,
	value_added_fee: Option<(AccountIdOf<T>, BalanceOf<T, T::MultiCurrency>)>,
}

impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config + pallet_transaction_payment::Config,
{
	type Balance = BalanceOf<T, T::MultiCurrency>;

	// TODO: deal with correct liquidity info logic
	type LiquidityInfo = Option<MultiCurrencyPayout<T>>;

	fn withdraw_fee(
		who: &T::AccountId,
		call: &CallOf<T>,
		dispatch_info: &DispatchInfoOf<CallOf<T>>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		// no fees aquired
		if fee.is_zero() {
			return Ok(None)
		}

		let fallback_asset = T::DefaultFeeAsset::get();

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};
		// try carrier first, no need to withdraw if carrier can handle the job.
		if let Some((
			carrier_address,
			carrier_data,
			value,
			max_gas,
			storage_deposit_limit,
			post_transfer,
		)) = T::IsCarrierAttachedCall::is_call(call)
		{
			let amount = T::FeeMeasure::measure(&fallback_asset, fee + tip)?;

			let mut obtained = T::Carrier::execute_carrier(
				who,
				&carrier_address,
				carrier_data.clone(),
				value,
				max_gas,
				storage_deposit_limit,
				amount,
				post_transfer,
			)
			.map_err(|e| {
				log::debug!("{:?}", e);
				TransactionValidityError::from(InvalidTransaction::Payment)
			})?;

			let over_collected = obtained.saturating_sub(amount);

			// return over collected immediately
			if !over_collected.is_zero() {
				T::FeeDispatch::refund(who, &fallback_asset, &over_collected).map_err(|e| {
					log::debug!("{:?}", e);
					TransactionValidityError::from(InvalidTransaction::Payment)
				})?;

				obtained.saturating_reduce(over_collected);
			};

			let payout_info = MultiCurrencyPayout {
				source_asset_id: fallback_asset,
				request_amount_native: fee + tip,
				withdrawn_source_amount: obtained,
				value_added_fee: T::IsFeeSharingCall::is_call(call),
			};

			let pallet_acc: AccountIdOf<T> = T::PalletId::get().try_into_account().unwrap();

			// burn obtained amount collected from PalletId
			T::FeeDispatch::withdraw(&pallet_acc, &fallback_asset, &obtained, &withdraw_reason)
				.map_err(|e| {
					log::debug!("{:?}", e);
					TransactionValidityError::from(InvalidTransaction::Payment)
				})?;

			Pallet::<T>::deposit_event(Event::<T>::CarrierExecute {
				carrier_address,
				carrier_data,
				post_transfer,
			});

			Pallet::<T>::deposit_event(Event::<T>::FeeWithdrawn {
				currency: fallback_asset,
				amount: obtained,
			});

			return Ok(Some(payout_info))
		}

		let preferred_fee_asset = Self::account_fee_source_priority(who).unwrap_or(fallback_asset);

		// check if preferenced fee source is both listed and accepted
		T::FeeSource::listed(&preferred_fee_asset)
			.and_then(|_| T::FeeSource::accepted(who, &preferred_fee_asset))
			.map_err(|e| {
				log::debug!("{:?}", e);
				TransactionValidityError::from(InvalidTransaction::Payment)
			})?;

		let amount = T::FeeMeasure::measure(&preferred_fee_asset, fee + tip)?;

		// try alt_token path first
		if T::FeeDispatch::withdraw(who, &preferred_fee_asset, &amount, &withdraw_reason).is_ok() {
			let payout_info = MultiCurrencyPayout {
				source_asset_id: preferred_fee_asset,
				request_amount_native: fee + tip,
				withdrawn_source_amount: amount,
				value_added_fee: T::IsFeeSharingCall::is_call(call),
			};

			Pallet::<T>::deposit_event(Event::<T>::FeeWithdrawn {
				currency: preferred_fee_asset,
				amount,
			});

			return Ok(Some(payout_info))
		}

		// retry using fallback if alt_token failed
		if (preferred_fee_asset != fallback_asset) &&
			T::FeeDispatch::withdraw(who, &fallback_asset, &(fee + tip), &withdraw_reason)
				.is_ok()
		{
			Pallet::<T>::deposit_event(Event::<T>::FallbackToNative);
			let fallback_amount = T::FeeMeasure::measure(&fallback_asset, fee + tip)?;

			let payout_info = MultiCurrencyPayout {
				source_asset_id: fallback_asset,
				request_amount_native: fee + tip,
				withdrawn_source_amount: fallback_amount,
				value_added_fee: T::IsFeeSharingCall::is_call(call),
			};

			Pallet::<T>::deposit_event(Event::<T>::FeeWithdrawn {
				currency: fallback_asset,
				amount: fallback_amount,
			});

			return Ok(Some(payout_info))
		}

		Err(InvalidTransaction::Payment.into())
	}

	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &DispatchInfoOf<CallOf<T>>,
		post_info: &PostDispatchInfoOf<CallOf<T>>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), frame_support::unsigned::TransactionValidityError> {
		if let Some(MultiCurrencyPayout {
			source_asset_id,
			request_amount_native,
			withdrawn_source_amount,
			value_added_fee,
		}) = already_withdrawn
		{
			let mut corrected_withdrawn = withdrawn_source_amount;

			// overcharged amount in native
			let overcharged_amount_native = request_amount_native.saturating_sub(corrected_fee);

			if !overcharged_amount_native.is_zero() {
				let amounts_source =
					T::FeeMeasure::measure(&source_asset_id, overcharged_amount_native)?;

				// it's possible refund failed, due to below E.D or routing temporary not possible
				if let Ok(refunded) = T::FeeDispatch::refund(who, &source_asset_id, &amounts_source)
				{
					corrected_withdrawn.saturating_reduce(refunded);
					Pallet::<T>::deposit_event(Event::<T>::FeeRefunded {
						currency: source_asset_id,
						amount: refunded,
					});
				}
			}

			// calculate tip amount in target token
			let tip_amount_source = T::FeeMeasure::measure(&source_asset_id, tip)?;

			// reduce tip from total amount
			corrected_withdrawn.saturating_reduce(tip_amount_source);

			// splits the remaining balances between all needed parties.
			T::FeeDispatch::post_info_correction(
				&source_asset_id,
				&tip_amount_source,
				&corrected_withdrawn,
				&value_added_fee,
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
