use orml_traits::{DataProvider, DefaultPriceProvider, PriceProvider};
use primitives::{Balance, CurrencyId, Price, TokenId};
use sp_runtime::{
	traits::{CheckedDiv, CheckedMul},
	FixedPointNumber,
};
use traits::fee::FeeMeasure;

use crate::mock::*;

#[test]
fn test_measure() {
	ExtBuilder::default().build().execute_with(|| {
		let native_required = 1000 as Balance;

		assert_eq!(
			FeeMeasurement::measure(&CurrencyId::NativeToken(TokenId::Laguna), native_required),
			Ok(native_required)
		);

		let ratio = <Runtime as crate::Config>::PrepaidConversionRate::get();

		assert_eq!(
			FeeMeasurement::measure(&CurrencyId::NativeToken(TokenId::FeeToken), native_required)
				.ok(),
			ratio.checked_mul_int(native_required)
		);
	});
}
