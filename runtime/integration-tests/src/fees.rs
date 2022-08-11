#[cfg(test)]
mod tests {

	use crate::*;
	use codec::Encode;
	use frame_support::{
		assert_ok,
		dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
		sp_runtime::traits::SignedExtension,
		weights::Pays,
	};
	use laguna_runtime::{
		constants::LAGUNAS, Currencies, FeeEnablement, FeeMeasurement, FluentFee, Origin,
		PrepaidFee, TransactionPayment, Treasury,
	};
	use pallet_transaction_payment::ChargeTransactionPayment;

	use sp_runtime::{FixedPointNumber, FixedU128};
	use traits::fee::FeeMeasure;

	fn step<'a>(
		accs: impl Iterator<Item = &'a AccountId>,
		asset_id: CurrencyId,
		mut action: impl FnMut(),
	) -> Vec<Balance> {
		action();

		accs.map(|acc| Currencies::free_balance(acc.clone(), asset_id))
			.collect::<Vec<_>>()
	}

	fn balance_of(who: AccountId, asset_id: CurrencyId) -> Balance {
		Currencies::free_balance(who, asset_id)
	}

	#[test]
	fn test_basic_fee_payout() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				let alice_init = balance_of(ALICE, NATIVE_CURRENCY_ID);

				// prepare a call
				let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
					to: BOB,
					currency_id: NATIVE_CURRENCY_ID,
					balance: LAGUNAS,
				});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

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

				let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

				// // TODO: refund logic and payout to validator etc should work
				assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
					Some(pre),
					&info,
					&post,
					len,
					&Ok(()),
				));

				let alice_refunded = balance_of(ALICE, NATIVE_CURRENCY_ID);

				assert_eq!(alice_init - fee - LAGUNAS, alice_refunded);
			});
	}

	#[test]
	fn test_alt_fee_path() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				// allow paying fee with FEE_TOKEN
				assert_ok!(FeeEnablement::onboard_asset(Origin::root(), FEE_TOKEN, true));

				// ALICE use FEE_TOKEN as default fee_source
				assert_ok!(FluentFee::set_default(Origin::signed(ALICE), FEE_TOKEN));
				assert_eq!(FluentFee::account_fee_source_priority(&ALICE), Some(FEE_TOKEN));

				assert_ok!(PrepaidFee::prepaid_native(Origin::signed(ALICE), LAGUNAS));
				assert_eq!(Currencies::free_balance(ALICE, FEE_TOKEN), LAGUNAS);

				// prepare a call
				let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
					to: ALICE,
					currency_id: NATIVE_CURRENCY_ID,
					balance: LAGUNAS,
				});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

				let alice_init = Currencies::free_balance(ALICE, FEE_TOKEN);

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

				let fee_in_alt =
					<Runtime as pallet_fluent_fee::Config>::FeeMeasure::measure(&FEE_TOKEN, fee)
						.expect("unable to get conversion rate for target token");

				let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

				// TODO: refund logic and payout to validator etc should work
				assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
					Some(pre),
					&info,
					&post,
					len,
					&Ok(()),
				));

				let alice_post = Currencies::free_balance(ALICE, FEE_TOKEN);
				assert_eq!(alice_init, alice_post + fee_in_alt);

				let treasury_account = Treasury::account_id();
				let to_treasury = FixedU128::saturating_from_rational(49_u128, 100_u128);
				let expected_gain = to_treasury.saturating_mul_int(fee_in_alt);

				assert_eq!(
					Currencies::free_balance(treasury_account, NATIVE_CURRENCY_ID),
					expected_gain
				);
			});
	}

	#[test]
	fn test_beneficiary() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				let treasury_ratio = FixedU128::saturating_from_rational(49_u128, 100_u128);
				let beneficiary_ratio = FixedU128::saturating_from_rational(2_u128, 100_u128);

				let treasury_acc = Treasury::account_id();
				let beneficiary_acc = EVA;

				// prepare a call
				let inner_call =
					laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
						to: ALICE,
						currency_id: NATIVE_CURRENCY_ID,
						balance: LAGUNAS,
					});

				let call =
					laguna_runtime::Call::FluentFee(pallet_fluent_fee::Call::fee_sharing_wrapper {
						beneficiary: Some(beneficiary_acc.clone()),
						call: Box::new(inner_call),
					});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

				let treasury_init =
					Currencies::free_balance(treasury_acc.clone(), NATIVE_CURRENCY_ID);
				let beneficiary_init =
					Currencies::free_balance(beneficiary_acc.clone(), NATIVE_CURRENCY_ID);

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

				// nothing should have changed before post_correction AKA payout was done.
				assert_eq!(
					treasury_init,
					Currencies::free_balance(treasury_acc.clone(), NATIVE_CURRENCY_ID)
				);
				assert_eq!(
					beneficiary_init,
					Currencies::free_balance(beneficiary_acc.clone(), NATIVE_CURRENCY_ID)
				);

				let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

				// TODO: refund logic and payout to validator etc should work
				assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
					Some(pre),
					&info,
					&post,
					len,
					&Ok(()),
				));

				let treasury_reward = treasury_ratio.saturating_mul_int(fee);
				let beneficiary_reward = beneficiary_ratio.saturating_mul_int(fee);

				assert_eq!(
					treasury_init + treasury_reward,
					Currencies::free_balance(treasury_acc, NATIVE_CURRENCY_ID)
				);

				assert_eq!(
					beneficiary_init + beneficiary_reward,
					Currencies::free_balance(beneficiary_acc, NATIVE_CURRENCY_ID)
				);
			});
	}
}
