//! ## pallet-fee-measurement
//!
//! This module provides price feeds for estimated required target assets when trying to pay
//! alternative fee sources

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, sp_runtime};
use orml_traits::price::PriceProvider;
use sp_runtime::{FixedPointNumber, FixedPointOperand};
use traits::fee::FeeMeasure;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Rate: FixedPointNumber;
		type Balance: FixedPointOperand;
		type CurrencyId: PartialEq + Copy + Clone;

		type NativeToken: Get<Self::CurrencyId>;
		type PrepaidToken: Get<Self::CurrencyId>;

		/// provide prepaid token's conversion rate, incentivice user to opt in with prepaid
		type PrepaidConversionRate: Get<Self::Rate>;

		/// for non-native fee sources, market-based or oracle-based approach are required to
		/// provide good convert rate
		type AltConversionRate: PriceProvider<Self::CurrencyId, Self::Rate>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
}

// associate types are fixed here, since we're creating a consuming pallet
impl<T: Config> FeeMeasure for Pallet<T> {
	type AssetId = T::CurrencyId;
	type Balance = T::Balance;

	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, TransactionValidityError> {
		if *id == T::NativeToken::get() {
			return Ok(balance)
		}

		if *id == T::PrepaidToken::get() {
			let native_to_prepaid_ratio = T::PrepaidConversionRate::get();

			return native_to_prepaid_ratio
				.checked_mul_int(balance)
				.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))
		}

		Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
	}
}
