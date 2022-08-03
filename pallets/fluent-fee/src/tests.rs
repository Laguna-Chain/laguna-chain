//! Unit test for the fluent-fee pallet

use super::pallet;
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

#[test]
fn test_fee_sharing_beneficiary_works() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000_000_000_000),
			(BOB, NATIVE_CURRENCY_ID, 1000_000_000_000),
			(EVA, NATIVE_CURRENCY_ID, 1000_000_000_000),
		])
		.build()
		.execute_with(|| {
			// Prepare the call
			let call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100,
			});

			let eva_balance_before = Tokens::free_balance(NATIVE_CURRENCY_ID, &EVA);
			// Construct the wrapped call. This is needed to trigger the pre_dispatch() from the
			// SignedExtension in order to charge fees.
			let wrapped_call = Call::FluentFee(pallet::Call::fee_sharing_wrapper {
				call: Box::new(call),
				beneficiary: Some(EVA),
			});
			// get the call length and info
			let len = wrapped_call.encoded_size();
			let info = wrapped_call.get_dispatch_info();
			ChargeTransactionPayment::<Runtime>::from(0)
				.pre_dispatch(&ALICE, &wrapped_call, &info, len)
				.expect("should pass");

			// Execute the wrapped call
			assert_ok!(wrapped_call.dispatch(Origin::signed(ALICE)));
			// EVA should have recieved tokens equivalent to unit weight
			let eva_balance_after = Tokens::free_balance(NATIVE_CURRENCY_ID, &EVA);
			assert!(eva_balance_after > eva_balance_before);
		})
}

#[test]
fn test_fee_sharing_none_works() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000_000_000_000),
			(BOB, NATIVE_CURRENCY_ID, 1000_000_000_000),
			(EVA, NATIVE_CURRENCY_ID, 1000_000_000_000),
		])
		.build()
		.execute_with(|| {
			// Prepare the call
			let call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100,
			});
			// Construct the wrapped call. This is needed to trigger the pre_dispatch() from the
			// SignedExtension in order to charge fees.
			let wrapped_call = Call::FluentFee(pallet::Call::fee_sharing_wrapper {
				call: Box::new(call),
				beneficiary: None,
			});
			// get the call length and info
			let len = wrapped_call.encoded_size();
			let info = wrapped_call.get_dispatch_info();
			ChargeTransactionPayment::<Runtime>::from(0)
				.pre_dispatch(&ALICE, &wrapped_call, &info, len)
				.expect("should pass");

			// Execute the wrapped call
			assert_ok!(wrapped_call.dispatch(Origin::signed(ALICE)));
		})
}
