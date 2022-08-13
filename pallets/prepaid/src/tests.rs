use frame_support::{assert_ok, sp_runtime};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::AccountId;
use sp_runtime::traits::AccountIdConversion;

use super::mock::*;

#[test]
fn test_prepaid() {
	ExtBuilder::default()
		.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_ok!(PrepaidFee::prepaid_native(Origin::signed(ALICE), 10000));

			let pallet_account: AccountId =
				<Runtime as crate::Config>::PalletId::get().into_account();

			assert_eq!(
				Tokens::reserved_balance(NATIVE_CURRENCY_ID, &pallet_account),
				Tokens::free_balance(FEE_CURRENCY_ID, &ALICE),
			);

			assert_ok!(PrepaidFee::unserve_to(BOB, 10000));
			assert_eq!(Tokens::reserved_balance(NATIVE_CURRENCY_ID, &pallet_account), 0);
			assert_eq!(Tokens::free_balance(NATIVE_CURRENCY_ID, &BOB), 10000);

			// currently no path for burning fee token exists, so the total supply need to be
			// manually updated for test.

			assert_ok!(Tokens::withdraw(FEE_CURRENCY_ID, &ALICE, 10000));

			assert!(PrepaidFee::prepaid_native(Origin::signed(ALICE), 200_000).is_err());
		})
}
