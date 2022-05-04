//! Unit test for the fluent-fee pallet

use super::*;

use frame_support::{assert_ok, dispatch::DispatchInfo};
use mock::*;
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::traits::SignedExtension;

#[test]
fn test_default_fee_asset() {
	ExtBuilder::default()
		.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 100_000_000)])
		.build()
		.execute_with(|| {
			assert_eq!(Currencies::free_balance(NATIVE_CURRENCY_ID, &ALICE), 100_000_000);
			assert_eq!(NATIVE_CURRENCY_ID, FluentFee::default_fee_source());
			assert_ok!(Currencies::withdraw(NATIVE_CURRENCY_ID, &ALICE, 100_000_00));
			assert_eq!(Currencies::free_balance(NATIVE_CURRENCY_ID, &ALICE), 90_000_000);

			let call = mock::Call::Currencies(orml_currencies::Call::transfer {
				dest: todo!(),
				currency_id: todo!(),
				amount: todo!(),
			});

			ChargeTransactionPayment::<Runtime>::from(0).pre_dispatch(
				&ALICE,
				&call,
				&DispatchInfo { weight: 100, ..Default::default() },
				10,
			);
		});
}
