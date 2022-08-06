use frame_support::assert_ok;
use primitives::{CurrencyId, TokenId};
use traits::fee::FeeSource;

use crate::mock::*;

const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
const FEETOKEN_ID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);

#[test]
fn test_listed() {
	ExtBuilder::default()
		.enabled(vec![(NATIVE_CURRENCY_ID, true)])
		.build()
		.execute_with(|| {
			assert!(crate::FeeAssets::<Runtime>::get(NATIVE_CURRENCY_ID).unwrap_or_default());
			assert!(!crate::FeeAssets::<Runtime>::get(FEETOKEN_ID).unwrap_or_default());

			assert_ok!(<FeeEnablement as FeeSource>::listed(&CurrencyId::NativeToken(
				TokenId::Laguna
			),));

			assert!(<FeeEnablement as FeeSource>::listed(&CurrencyId::NativeToken(
				TokenId::FeeToken
			),)
			.is_err());
		});
}

#[test]
fn test_onboarding() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(!crate::FeeAssets::<Runtime>::get(NATIVE_CURRENCY_ID).unwrap_or_default());

		assert!(FeeEnablement::listed(&NATIVE_CURRENCY_ID).is_err());

		assert_ok!(FeeEnablement::onboard_asset(Origin::root(), NATIVE_CURRENCY_ID, false));
		assert_eq!(crate::FeeAssets::<Runtime>::get(NATIVE_CURRENCY_ID), Some(false));

		assert!(<FeeEnablement as FeeSource>::listed(&NATIVE_CURRENCY_ID).is_err());

		assert_ok!(FeeEnablement::enable_asset(Origin::root(), NATIVE_CURRENCY_ID));
		assert_ok!(<FeeEnablement as FeeSource>::listed(&NATIVE_CURRENCY_ID));
	});
}

#[test]
fn test_accepted() {
	ExtBuilder::default()
		.enabled(vec![(FEETOKEN_ID, true)])
		.build()
		.execute_with(|| {
			assert!(crate::FeeAssets::<Runtime>::get(FEETOKEN_ID).unwrap_or_default());
			assert_ok!(FeeEnablement::listed(&FEETOKEN_ID));

			// FeeToken shouldn't be allowed due to lack of liquidity in our setup
			assert!(FeeEnablement::accepted(&ALICE, &FEETOKEN_ID).is_err());

			// BOB was mandatory blacklisted
			assert!(FeeEnablement::accepted(&BOB, &FEETOKEN_ID).is_err());

			// manually raise the token for it to be in a healthy state
			assert_ok!(Tokens::set_balance(Origin::root(), ALICE, FEETOKEN_ID, 1_000_000, 0));
			assert_ok!(FeeEnablement::accepted(&ALICE, &FEETOKEN_ID));

			// BOB was mandatory blacklisted
			assert!(FeeEnablement::accepted(&BOB, &FEETOKEN_ID).is_err());

			// manually raise the token for it to be in a unhealthy state
			assert_ok!(Tokens::set_balance(Origin::root(), ALICE, FEETOKEN_ID, 0, 0));

			// unhealthy asset can be included in fee payout
			assert!(FeeEnablement::accepted(&ALICE, &FEETOKEN_ID).is_err());
		});
}
