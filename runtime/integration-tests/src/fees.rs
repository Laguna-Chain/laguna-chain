#[cfg(test)]
mod tests {

	use crate::*;
	use codec::Encode;
	use frame_support::{
		assert_ok,
		dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo, Weight},
		sp_runtime::traits::SignedExtension,
		weights::Pays,
	};
	use laguna_runtime::{
		constants::LAGUNAS, Currencies, FeeEnablement, FeeMeasurement, FluentFee, Origin,
		TransactionPayment,
	};
	use pallet_transaction_payment::ChargeTransactionPayment;

	use traits::fee::FeeMeasure;

	#[test]
	fn test_basic_fee_payout() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				// prepare a call
				let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
					to: BOB,
					currency_id: NATIVE_CURRENCY_ID,
					balance: LAGUNAS,
				});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

				let pre_dispatch_amount = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

				// pre_dispatch will trigger the SignedExtension
				// via `TransactionPayment --> OnchargeTransaction --> FluentFee`
				// we can test fee chargin logic by calling validate once
				let pre = ChargeTransactionPayment::<Runtime>::from(0)
					.pre_dispatch(&ALICE, &call, &info, len)
					.expect("should pass");

				// calculate actual fee with all the parameter including base_fee, length_fee and
				// byte_multiplier etc.
				let fee = TransactionPayment::compute_actual_fee(
					len as u32,
					&info,
					&PostDispatchInfo { actual_weight: Some(info.weight), pays_fee: Pays::Yes },
					0,
				);

				let post_dispatch_amount = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

				assert_eq!(pre_dispatch_amount, post_dispatch_amount + fee);

				let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

				// TODO: refund logic and payout to validator etc should work
				assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
					Some(pre),
					&info,
					&post,
					len,
					&Ok(()),
				));

				// expected final states
				assert_eq!(
					Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID),
					(10 - 1) * LAGUNAS - fee
				);
			});
	}

	#[test]
	fn test_alt_fee_path() {
		ExtBuilder::default()
			.balances(vec![
				(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS),
				(ALICE, FEE_TOKEN, 10 * LAGUNAS),
			])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				// allow paying fee with FEE_TOKEN
				assert_ok!(FeeEnablement::onboard_asset(Origin::root(), FEE_TOKEN, true));

				// ALICE use FEE_TOKEN as default fee_source
				assert_ok!(FluentFee::set_default(Origin::signed(ALICE), FEE_TOKEN));
				assert_eq!(FluentFee::account_fee_source_priority(&ALICE), Some(FEE_TOKEN));

				// prepare a call
				let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
					to: ALICE,
					currency_id: NATIVE_CURRENCY_ID,
					balance: LAGUNAS,
				});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

				let pre_dispatch_amount = Currencies::free_balance(ALICE, FEE_TOKEN);

				// pre_dispatch will trigger the SignedExtension
				// via `TransactionPayment --> OnchargeTransaction --> FluentFee`
				// we can test fee chargin logic by calling validate once
				let pre = ChargeTransactionPayment::<Runtime>::from(0)
					.pre_dispatch(&ALICE, &call, &info, len)
					.expect("should pass");

				// calculate actual fee with all the parameter including base_fee, length_fee and
				// byte_multiplier etc.
				let fee = TransactionPayment::compute_actual_fee(
					len as u32,
					&info,
					&PostDispatchInfo { actual_weight: Some(info.weight), pays_fee: Pays::Yes },
					0,
				);

				let post_dispatch_amount = Currencies::free_balance(ALICE, FEE_TOKEN);

				let targeted =
					FeeMeasurement::measure(&FEE_TOKEN, fee).expect("unable to get convert rate");
				assert_eq!(pre_dispatch_amount - post_dispatch_amount, targeted);

				let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

				// TODO: refund logic and payout to validator etc should work
				assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
					Some(pre),
					&info,
					&post,
					len,
					&Ok(()),
				));

				// assert_eq!(pre_dispatch_amount, post_dispatch_amount + fee);
			});
	}
}
