#![cfg(test)]

use std::str::FromStr;

use crate::{contracts::consume_native_token::deploy_system_contract, *};
use codec::Encode;
use frame_support::{
	assert_ok,
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	sp_runtime::traits::SignedExtension,
	traits::Get,
	weights::Pays,
};
use laguna_runtime::{
	constants::LAGUNAS, Currencies, FeeEnablement, FluentFee, Origin, PrepaidFee, SudoContracts,
	Tokens, TransactionPayment, Treasury,
};
use pallet_transaction_payment::ChargeTransactionPayment;

use sp_core::{Bytes, U256};
use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, FixedU128};
use traits::fee::FeeMeasure;

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
			let inner_call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
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

			let treasury_init = Currencies::free_balance(treasury_acc.clone(), NATIVE_CURRENCY_ID);
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

#[test]
fn test_prepaid_insufficent() {
	ExtBuilder::default()
		.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
		.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true), (FEE_TOKEN, true)])
		.build()
		.execute_with(|| {
			// ALICE use FEE_TOKEN as default fee_source
			assert_ok!(FluentFee::set_default(Origin::signed(ALICE), FEE_TOKEN));
			assert_eq!(FluentFee::account_fee_source_priority(&ALICE), Some(FEE_TOKEN));

			assert_ok!(PrepaidFee::prepaid_native(Origin::signed(ALICE), LAGUNAS));
			assert_eq!(Currencies::free_balance(ALICE, FEE_TOKEN), LAGUNAS);

			let treasury_ratio = FixedU128::saturating_from_rational(49_u128, 100_u128);
			let treasury_acc = Treasury::account_id();
			let treasury_init = Currencies::free_balance(treasury_acc.clone(), NATIVE_CURRENCY_ID);

			let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
				to: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				balance: LAGUNAS,
			});

			let len = call.encoded_size();
			let info = call.get_dispatch_info();

			// clean all fee_tokens
			assert_ok!(<Tokens as orml_traits::MultiCurrency<AccountId>>::withdraw(
				FEE_TOKEN, &ALICE, LAGUNAS
			));

			let alice_pre_charged = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

			// should fallback to native token if preferred token is not enough
			let pre = ChargeTransactionPayment::<Runtime>::from(0)
				.pre_dispatch(&ALICE, &call, &info, len)
				.expect("unable to withdrawn");

			let fee = TransactionPayment::compute_actual_fee(
				len as u32,
				&info,
				&PostDispatchInfo { actual_weight: Some(info.weight), pays_fee: Pays::Yes },
				0,
			);

			assert_eq!(
				alice_pre_charged,
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID) + fee
			);

			let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

			assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
				Some(pre),
				&info,
				&post,
				len,
				&Ok(()),
			));

			let treasury_reward = treasury_ratio.saturating_mul_int(fee);

			assert_eq!(
				treasury_init + treasury_reward,
				Currencies::free_balance(treasury_acc, NATIVE_CURRENCY_ID)
			);
		});
}

#[test]
fn test_with_carrier() {
	let deploying_key = <Runtime as pallet_system_contract_deployer::Config>::PalletId::get()
		.try_into_account()
		.expect("Invalid PalletId");
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS),
			(deploying_key, NATIVE_CURRENCY_ID, 10 * LAGUNAS),
		])
		.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
		.build()
		.execute_with(|| {
			let (treasury_ratio, _, _) =
				<Runtime as pallet_fluent_fee::Config>::PayoutSplits::get();

			let treasury_acc = Treasury::account_id();

			let blob = std::fs::read(
				"./contracts-data/ink/native_fungible_token/dist/native_fungible_token.wasm",
			)
			.unwrap();

			let mut sel_constructor = Bytes::from_str("0x45fd0674")
				.map(|v| v.to_vec())
				.expect("unable to parse selector");

			0_u32.encode_to(&mut sel_constructor);

			let token_addr = deploy_system_contract(blob, sel_constructor);

			// prepare a call
			let inner_call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
				to: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				balance: LAGUNAS,
			});

			let pallet_acc: AccountId = <Runtime as pallet_fluent_fee::Config>::PalletId::get()
				.try_into_account()
				.unwrap();

			let mut carrier_data = Bytes::from_str("0xa9059cbb").map(|v| v.to_vec()).unwrap();

			(pallet_acc, U256::from(LAGUNAS)).encode_to(&mut carrier_data);

			let call = laguna_runtime::Call::FluentFee(pallet_fluent_fee::Call::carrier_wrapper {
				carrier: token_addr,
				carrier_data,
				call: Box::new(inner_call),
			});

			let len = call.encoded_size();
			let info = call.get_dispatch_info();

			let treasury_init = Currencies::free_balance(treasury_acc.clone(), NATIVE_CURRENCY_ID);
			let acc_init = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

			// pre_dispatch will trigger the SignedExtension
			// via `TransactionPayment --> OnchargeTransaction --> FluentFee`
			// we can test fee chargin logic by calling validate once
			let pre = ChargeTransactionPayment::<Runtime>::from(0)
				.pre_dispatch(&ALICE, &call, &info, len)
				.expect("should pass");

			let acc_charged = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);
			assert_eq!(acc_init, acc_charged + LAGUNAS);

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
			let post = call.dispatch(Origin::signed(ALICE)).expect("should be dispatched");

			let acc_call = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

			assert_eq!(acc_charged, acc_call + LAGUNAS);

			assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
				Some(pre),
				&info,
				&post,
				len,
				&Ok(()),
			));

			let acc_refunded = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);
			assert_eq!(acc_init - LAGUNAS - fee, acc_refunded);

			let treasury_reward = treasury_ratio.saturating_mul_int(fee);

			assert_eq!(
				treasury_init + treasury_reward,
				Currencies::free_balance(treasury_acc, NATIVE_CURRENCY_ID)
			);
		});
}
