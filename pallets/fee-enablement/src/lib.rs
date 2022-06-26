//! # pallet-fee-enablement
//!
//! This pallet took part of the fee-distribution pipeline where inclusion of an asset is managed
//! and checked. This pallet implement the `traits::fee::FeeSource` trait which controls wether an
//! asset is allowed to took part in fee payout.

#![cfg_attr(not(feature = "std"), no_std)]

// +++++++++++
// + imports +
// +++++++++++

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use orml_traits::MultiCurrency;
use primitives::CurrencyId;

use traits::fee::{Eligibility, FeeAssetHealth, FeeSource, InvalidFeeSource};

pub use pallet::*;
use weights::WeightInfo;

// +++++++++++
// + Aliases +
// +++++++++++

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<B, T> = <B as MultiCurrency<AccountIdOf<T>>>::Balance;

pub mod weights;

#[frame_support::pallet]
mod pallet {

	use traits::fee::{Eligibility, FeeAssetHealth};

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type AllowedOrigin: EnsureOrigin<Self::Origin>;

		/// determing whether an asset is allowed to be active by checking with chain
		/// conditions such as total staked or liquidity
		type HealthStatus: FeeAssetHealth<AssetId = CurrencyId>;

		/// whether an account met the condition to use an asset as fee source
		type Eligibility: Eligibility<AccountId = AccountIdOf<Self>, AssetId = CurrencyId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type FeeAssets<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, bool>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::onboard_asset())]
		pub fn onboard_asset(
			origin: OriginFor<T>,
			asset_id: CurrencyId,
			enabled: bool,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin.clone())?;

			FeeAssets::<T>::insert(asset_id, enabled);
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::enable_asset())]
		pub fn enable_asset(origin: OriginFor<T>, asset_id: CurrencyId) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin.clone())?;

			FeeAssets::<T>::mutate(asset_id, |val| *val = Some(true));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::disable_asset())]
		pub fn disable_asset(origin: OriginFor<T>, asset_id: CurrencyId) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin.clone())?;
			FeeAssets::<T>::mutate(asset_id, |val| *val = Some(false));

			Ok(())
		}
	}

	#[pallet::genesis_config]
	#[derive(Default)]
	pub struct GenesisConfig {
		pub enabled: Vec<(CurrencyId, bool)>,
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
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
	type AssetId = CurrencyId;

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
