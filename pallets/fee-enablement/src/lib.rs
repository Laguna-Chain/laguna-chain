//! # pallet-fee-enablement
//!
//! This pallet took part of the fee-distribution pipeline where inclusion of an asset is managed
//! and checked. This pallet implement the `traits::fee::FeeSource` trait which controls wether an
//! asset is allowed to took part in fee payout.

#![cfg_attr(not(feature = "std"), no_std)]

// +++++++
// imports
// +++++++

use frame_support::{pallet_prelude::*, sp_std::prelude::*};
use frame_system::pallet_prelude::*;

use orml_traits::MultiCurrency;

use traits::fee::{Eligibility, FeeAssetHealth, FeeSource, InvalidFeeSource};

pub use pallet::*;
use weights::WeightInfo;

#[cfg(test)]
pub mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
pub mod tests;

// +++++++
// Aliases
// +++++++

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<B, T> = <B as MultiCurrency<AccountIdOf<T>>>::Balance;
pub type CurrencyOf<T, C> = <C as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub mod weights;

#[frame_support::pallet]
mod pallet {

	use traits::fee::{Eligibility, FeeAssetHealth};

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type AllowedOrigin: EnsureOrigin<Self::Origin>;
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>>;

		/// determing whether an asset is allowed to be active by checking with chain
		/// conditions such as total staked or liquidity
		type HealthStatus: FeeAssetHealth<AssetId = CurrencyOf<Self, Self::MultiCurrency>>;

		/// whether an account met the condition to use an asset as fee source
		type Eligibility: Eligibility<
			AccountId = AccountIdOf<Self>,
			AssetId = CurrencyOf<Self, Self::MultiCurrency>,
		>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type FeeAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyOf<T, T::MultiCurrency>, bool>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::onboard_asset())]
		pub fn onboard_asset(
			origin: OriginFor<T>,
			asset_id: CurrencyOf<T, T::MultiCurrency>,
			enabled: bool,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin)?;

			FeeAssets::<T>::insert(asset_id, enabled);
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::enable_asset())]
		pub fn enable_asset(
			origin: OriginFor<T>,
			asset_id: CurrencyOf<T, T::MultiCurrency>,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin)?;

			FeeAssets::<T>::mutate(asset_id, |val| *val = Some(true));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::disable_asset())]
		pub fn disable_asset(
			origin: OriginFor<T>,
			asset_id: CurrencyOf<T, T::MultiCurrency>,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin)?;
			FeeAssets::<T>::mutate(asset_id, |val| *val = Some(false));

			Ok(())
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub enabled: Vec<(CurrencyOf<T, T::MultiCurrency>, bool)>,
	}

	#[cfg(feature = "std")]

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { enabled: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (asset_id, enabled) in &self.enabled {
				FeeAssets::<T>::insert(asset_id, enabled);
			}
		}
	}
}

impl<T> FeeSource for Pallet<T>
where
	T: Config,
{
	type AccountId = AccountIdOf<T>;
	type AssetId = CurrencyOf<T, T::MultiCurrency>;

	fn accepted(
		who: &Self::AccountId,
		id: &Self::AssetId,
	) -> Result<(), traits::fee::InvalidFeeSource> {
		T::HealthStatus::health_status(id)
			.map_err(|_| InvalidFeeSource::Inactive)
			.and_then(|_| {
				T::Eligibility::eligible(who, id).map_err(|_| InvalidFeeSource::Inactive)
			})?;

		log::debug!(target: "fee_enablement::fee_source", "{:?} accepted", id);
		Ok(())
	}

	fn listed(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		if FeeAssets::<T>::get(id).unwrap_or_default() {
			log::debug!(target: "fee_enablement::fee_source", "{:?} listed", id);

			Ok(())
		} else {
			Err(InvalidFeeSource::Unlisted)
		}
	}
}
