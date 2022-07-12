//! Unit test for the fluent-fee pallet

use crate::mock::{Call, *};
use codec::Encode;
use frame_support::{
	assert_ok,
	dispatch::{DispatchInfo, Dispatchable, GetDispatchInfo},
	traits::fungibles::Balanced,
};

use orml_traits::MultiCurrency;
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use primitives::{CurrencyId, TokenId};
use sp_runtime::traits::SignedExtension;
use traits::fee::FeeMeasure;

#[test]
fn test_charge_native() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000_000_000_000_000),
			// (ALICE, FEE_CURRENCY_ID, 1000_000_000_000),
		])
		.build()
		.execute_with(|| {
			let call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100,
			});

			let len = call.encoded_size();
			let info = call.get_dispatch_info();
			let fee = Payment::compute_fee(len as u32, &info, 0);

			assert_ok!(ChargeTransactionPayment::<Runtime>::from(0)
				.validate(&ALICE, &call, &info, len as _,));

			assert_ok!(call.dispatch(Origin::signed(ALICE)));

			assert_eq!(
				Tokens::free_balance(NATIVE_CURRENCY_ID, &ALICE),
				1000_000_000_000_000 - fee - 100
			);
		});
}

#[test]
fn test_charge_fee() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000_000_000_000),
			(ALICE, FEE_CURRENCY_ID, 1000_000_000_000),
		])
		.build()
		.execute_with(|| {
			// set default
			assert_ok!(FluentFee::set_default(
				Origin::signed(ALICE),
				CurrencyId::NativeToken(TokenId::FeeToken),
			));

			assert_eq!(FluentFee::account_fee_source_priority(&ALICE), Some(FEE_CURRENCY_ID));

			let call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100,
			});

			assert_ok!(ChargeTransactionPayment::<Runtime>::from(0).validate(
				&ALICE,
				&call,
				&call.get_dispatch_info(),
				call.encoded_size(),
			));

			let len = call.encoded_size();
			let info = call.get_dispatch_info();
			let fee = Payment::compute_fee(len as u32, &info, 0);

			let discounted = <Runtime as crate::Config>::FeeMeasure::measure(&FEE_CURRENCY_ID, fee)
				.expect("received target amount");

			assert_eq!(
				Tokens::free_balance(FEE_CURRENCY_ID, &ALICE),
				1000_000_000_000 - discounted
			);
		});
}
