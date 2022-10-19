use crate::{Currencies, Runtime};
use frame_system::EnsureRoot;
use primitives::{AccountId, CurrencyId};
use traits::fee::{Eligibility, FeeAssetHealth};

impl pallet_fee_enablement::Config for Runtime {
	type MultiCurrency = Currencies;
	type AllowedOrigin = EnsureRoot<AccountId>;

	type HealthStatus = DefaultImpl;

	type Eligibility = DefaultImpl;

	type WeightInfo = ();
}

pub struct DefaultImpl;

impl FeeAssetHealth for DefaultImpl {
	type AssetId = CurrencyId;

	fn health_status(asset_id: &Self::AssetId) -> Result<(), traits::fee::HealthStatusError> {
		match asset_id {
			CurrencyId::NativeToken(_) => Ok(()),
			CurrencyId::Erc20(_) => Err(traits::fee::HealthStatusError::Unavailable),
		}
	}
}

impl Eligibility for DefaultImpl {
	type AccountId = AccountId;

	type AssetId = CurrencyId;

	fn eligible(
		_who: &Self::AccountId,
		_asset_id: &Self::AssetId,
	) -> Result<(), traits::fee::EligibilityError> {
		Ok(())
	}
}
