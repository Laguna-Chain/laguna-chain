//! ## pallet-fee-measurement
//!
//! This module provides price feeds for estimated required target assets when trying to pay
//! alternative fee sources

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, sp_runtime};
use frame_system::pallet_prelude::*;
use orml_traits::price::PriceProvider;
use primitives::{Balance, CurrencyId, Price, TokenId};
use sp_runtime::{traits::CheckedMul, FixedPointNumber};
use traits::fee::FeeMeasure;

pub use pallet::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
mod pallet {
	use primitives::Price;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// provide prepaid token's conversion rate, incentivice user to opt in with prepaid
		type PrepaidConversionRate: Get<Price>;

		/// for non-native fee sources, market-based or oracle-based approach are required to
		/// provide good convert rate
		type AltConversionRate: PriceProvider<CurrencyId, Price>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
}

// associate types are fixed here, since we're creating a consuming pallet
impl<T: Config> FeeMeasure for Pallet<T> {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, TransactionValidityError> {
		match id {
			// no conversion needed for native token
			CurrencyId::NativeToken(TokenId::Laguna) => Ok(balance),
			// get conversion rate for pre-paid token
			CurrencyId::NativeToken(TokenId::FeeToken) => {
				let native_to_prepaid_ratio = T::PrepaidConversionRate::get();

				native_to_prepaid_ratio
					.checked_mul_int(balance)
					.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))
			},
			// contract based assets's are not enabled for now
			// CurrencyId::NativeToken(_) | CurrencyId::Erc20(_) => {
			// 	let native_to_alt_ratio =
			// 		T::AltConversionRate::get_price(CurrencyId::NativeToken(TokenId::Laguna), *id)
			// 			.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			// 	native_to_alt_ratio
			// 		.checked_mul_int(balance)
			// 		.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))
			// },
			_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Payment)),
		}
	}
}
