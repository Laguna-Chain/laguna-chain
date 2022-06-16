use crate::Runtime;
use frame_system::EnsureRoot;
use primitives::{AccountId, CurrencyId};
use traits::fee::{Eligibility, FeeAssetHealth};

impl pallet_fee_enablement::Config for Runtime {
	type AllowedOrigin = EnsureRoot<AccountId>;

	type HealthStatus = DefaultImpl;

	type Eligibility = DefaultImpl;
}

pub struct DefaultImpl;

impl FeeAssetHealth for DefaultImpl {
	type AssetId = CurrencyId;

	fn health_status(asset_id: &Self::AssetId) -> Result<(), traits::fee::HealthStatusError> {
		Ok(())
	}
}

impl Eligibility for DefaultImpl {
	type AccountId = AccountId;

	type AssetId = CurrencyId;

	fn eligible(
		who: &Self::AccountId,
		asset_id: &Self::AssetId,
	) -> Result<(), traits::fee::EligibilityError> {
		Ok(())
	}
}
