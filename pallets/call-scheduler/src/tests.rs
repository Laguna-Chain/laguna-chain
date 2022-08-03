//! Unit test for call-scheduler
use super::*;
use crate::{
	mock::{Call, Event, *},
	tests::test_utils::charge_tx_fee,
};

use traits::currencies::TokenAccess;

use frame_support::{assert_ok, dispatch::DispatchInfo};
use pallet_transaction_payment::ChargeTransactionPayment;
use primitives::{AccountId, CurrencyId};
use sp_core::{Bytes, Hasher, U256};
use sp_runtime::{
	traits::{BlakeTwo256, Hash, SignedExtension},
	AccountId32,
};

mod test_utils;
use test_utils::*;

#[test]
fn test_erc20_fee_payment() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 10000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			let init_amount: u128 = 100000000000000000000000000000;
			let deployed = create_token(ALICE, "TKN", "TKN", init_amount);
			let CURRENCY_ID = CurrencyId::Erc20(deployed.clone().into());
			// let deployed: AccountId32 = AccountId32::from([1u8; 32]);

			assert_ok!(ContractAssets::register_asset(Origin::root(), deployed.clone(), true));
			// set the ERC20 as ALICE's prioritized gas fee source
			assert_ok!(FluentFee::set_default(Origin::signed(ALICE), CURRENCY_ID.clone()));
			// prepare a call
			let call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100000,
			});

			let len = call.encoded_size();
			let info = call.get_dispatch_info();

			let pre_dispatch_amount = Currencies::free_balance(ALICE, CURRENCY_ID.clone());

			// pre_dispatch will trigger the SignedExtension
			// via `TransactionPayment --> OnchargeTransaction --> FluentFee -> FeeDispatch`
			// we can test fee chargin logic by calling validate once
			let pre = ChargeTransactionPayment::<Runtime>::from(0)
				.pre_dispatch(&ALICE, &call, &info, len)
				.expect("should pass");

			// calculate actual fee with all the parameter including base_fee, length_fee and
			// byte_multiplier etc.
			let mut fee = TransactionPayment::compute_actual_fee(
				len as u32,
				&info,
				&PostDispatchInfo { actual_weight: Some(info.weight), pays_fee: Pays::Yes },
				0,
			);
			// The price conversion from native to erc20 is set to be 0.7 at the moment. So the
			// actual fee paid in erc20 would be fee * 0.7
			fee = fee.saturating_mul(70).saturating_div(100);

			let post_dispatch_amount = Currencies::free_balance(ALICE, CURRENCY_ID.clone());

			assert_eq!(pre_dispatch_amount, fee + post_dispatch_amount);

			let post = call.clone().dispatch(Origin::signed(ALICE)).expect(
				"should be
			dispatched",
			);

			// TODO: refund logic and payout to validator etc should work
			assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
				Some(pre),
				&info,
				&post,
				len,
				&Ok(()),
			));

			// expected final states
			assert_eq!(Currencies::free_balance(ALICE, CURRENCY_ID.clone()), init_amount - fee);
		})
}

#[test]
fn test_schedule_call_prepayment_works() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 10000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			// prepare a call
			let schedule_call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100000,
			});

			let id = BlakeTwo256::hash_of(&schedule_call).as_fixed_bytes().to_vec();
			let call = Call::Scheduler(pallet::Call::schedule_call {
				when: 5,
				call: Box::new(schedule_call),
				id,
				maybe_periodic: None,
				priority: 1,
			});
			let len = call.encoded_size();
			let info = call.get_dispatch_info();
			let init_locked_fund = 1000000000000000000;
			// Fund the schedule call balance for the origin (ALICE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(ALICE), init_locked_fund));
			// compute the fee charged for scheduling a call. NOTE: this fee does not include the
			// schedule_call initial deposit -- this amount is calculated and charged inside the
			// FeeDispatch::withdraw_fee()'s logic
			let fee = TransactionPayment::compute_fee(len as u32, &info, 0);
			let pre_dispatch_balance = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// pre_dispatch will trigger the SignedExtension
			// via `TransactionPayment --> OnchargeTransaction --> FluentFee -> FeeDispatch`
			// we can test fee chargin logic by calling validate once
			charge_tx_fee(ALICE.clone(), &call, &info, len.clone());
			// Dispatch the call
			assert_ok!(call.dispatch(Origin::signed(ALICE)));
			// Check if the schedule_call initial deposit has been charged
			let update_locked_funds = Scheduler::scheduled_locked_funds_balances(ALICE);
			assert!(update_locked_funds < init_locked_fund);
			let post_dispatch_balance = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			assert_eq!(pre_dispatch_balance, post_dispatch_balance + fee);
		})
}

#[test]
fn test_schedule_call_single_time_works() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 10000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			// Amount to transfer during scheduled calls
			let schedule_transfer_amount = 100000;
			// prepare a call
			let schedule_call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: schedule_transfer_amount.clone(),
			});

			let id = BlakeTwo256::hash_of(&schedule_call).as_fixed_bytes().to_vec();
			let call = Call::Scheduler(pallet::Call::schedule_call {
				when: 5,
				call: Box::new(schedule_call),
				id,
				maybe_periodic: None,
				priority: 1,
			});

			let len = call.encoded_size();
			let info = call.get_dispatch_info();
			let init_locked_fund = 1000000000000000000;
			// Fund the schedule call balance for the origin (ALICE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(ALICE), init_locked_fund));
			// compute the fee charged for scheduling a call. NOTE: this fee does not include the
			// schedule_call initial deposit -- this amount is calculated and charged inside the
			// FeeDispatch::withdraw_fee()'s logic
			let fee = TransactionPayment::compute_fee(len as u32, &info, 0);
			let pre_dispatch_balance = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// pre_dispatch will trigger the SignedExtension
			// via `TransactionPayment --> OnchargeTransaction --> FluentFee -> FeeDispatch`
			// we can test fee chargin logic by calling validate once
			charge_tx_fee(ALICE.clone(), &call, &info, len.clone());
			// Dispatch the call
			assert_ok!(call.dispatch(Origin::signed(ALICE)));
			// Check if the schedule_call initial deposit has been charged
			let update_locked_funds = Scheduler::scheduled_locked_funds_balances(ALICE);
			assert!(update_locked_funds < init_locked_fund);
			let post_dispatch_balance = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			assert_eq!(pre_dispatch_balance, post_dispatch_balance + fee);

			// ALICE's native token balance before jumping to block 5 and executing the scheduled
			// transfer call
			let alice_balance_before_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// Jump to #block 5
			jump_to_block(5);
			// ALICE's native token balance after executing the scheduled transfer call
			let alice_balance_after_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			assert_eq!(
				alice_balance_before_scheduled_call,
				schedule_transfer_amount + alice_balance_after_scheduled_call
			);
		})
}

#[test]
fn test_schedule_call_periodic_works() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			// Amount to transfer during scheduled calls
			let schedule_transfer_amount = 100000;
			// prepare a call
			let schedule_call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: schedule_transfer_amount.clone(),
			});

			let id = BlakeTwo256::hash_of(&schedule_call).as_fixed_bytes().to_vec();
			let init_locked_fund = 1000000000000000000;
			// Fund the schedule call balance for the origin (ALICE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(ALICE), init_locked_fund));

			assert_ok!(Scheduler::schedule_call(
				Origin::signed(ALICE),
				5,
				Box::new(schedule_call),
				id,
				Some((5, 3)), /* Schedule the call every 5 blocks for 3 (2 repeates, 1
				               * first-time scheduled) times */
				1
			));
			// ALICE's native token balance before jumping to block 5 and executing the scheduled
			// transfer call
			let alice_balance_before_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// Jump to #block 5
			jump_to_block(5);
			// ALICE's native token balance after executing the scheduled transfer call
			let alice_balance_after_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			assert_eq!(
				alice_balance_before_scheduled_call,
				schedule_transfer_amount + alice_balance_after_scheduled_call
			);

			// ALICE's native token balance before jumping to block 10 and executing the scheduled
			// transfer call
			let alice_balance_before_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// Jump to #block 10
			jump_to_block(10);
			// ALICE's native token balance after executing the scheduled transfer call
			let alice_balance_after_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			assert_eq!(
				alice_balance_before_scheduled_call,
				schedule_transfer_amount + alice_balance_after_scheduled_call
			);

			// ALICE's native token balance before jumping to block 15 and executing the scheduled
			// transfer call
			let alice_balance_before_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// Jump to #block 15
			jump_to_block(15);
			// ALICE's native token balance after executing the scheduled transfer call
			let alice_balance_after_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			assert_eq!(
				alice_balance_before_scheduled_call,
				schedule_transfer_amount + alice_balance_after_scheduled_call
			);
		})
}

#[test]
fn test_schedule_call_multiple_postponed_retries_refunded() {
	// This test won't work as the MaxScheduledCallsPerBlock doesn't enforce the condition and just
	// warns
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(CHARLIE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(EVA, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			// Amount to transfer during scheduled calls
			let schedule_transfer_amount = 100000;
			// prepare a call
			let schedule_call = Call::Tokens(orml_tokens::Call::transfer {
				dest: EVA,
				currency_id: NATIVE_CURRENCY_ID,
				amount: schedule_transfer_amount.clone(),
			});

			let id = BlakeTwo256::hash_of(&schedule_call).as_fixed_bytes().to_vec();
			let init_locked_fund = 1000000000000000000;
			// Fund the schedule call balance for the origin (ALICE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(ALICE), init_locked_fund));
			// Place the scheduled call for ALICE
			assert_ok!(Scheduler::schedule_call(
				Origin::signed(ALICE),
				5,
				Box::new(schedule_call.clone()),
				id.clone(),
				Some((5, 3)), /* Schedule the call every 5 blocks for 3 (2 repeates, 1
				               * first-time scheduled) times */
				1
			));

			// Fund the schedule call balance for the origin (BOB)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(BOB), init_locked_fund));
			// Place the scheduled call for ALICE
			assert_ok!(Scheduler::schedule_call(
				Origin::signed(BOB),
				5,
				Box::new(schedule_call.clone()),
				id.clone(),
				Some((5, 3)), /* Schedule the call every 5 blocks for 3 (2 repeates, 1
				               * first-time scheduled) times */
				1
			));

			// Fund the schedule call balance for the origin (CHARLIE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(CHARLIE), init_locked_fund));
			// Place the scheduled call for CHARLIE
			assert_ok!(Scheduler::schedule_call(
				Origin::signed(CHARLIE),
				5,
				Box::new(schedule_call),
				id,
				Some((5, 3)), /* Schedule the call every 5 blocks for 3 (2 repeates, 1
				               * first-time scheduled) times */
				3
			));
		})
}

#[test]
fn test_schedule_call_multiple_errors() {
	// Locks insufficient initial deposit such that the scheduled calls fails until crossing the
	// threshold and getting removed
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			// Amount to transfer during scheduled calls
			let schedule_transfer_amount = 10000000000000000000000000000000000000;
			// prepare a call
			let schedule_call = Call::Tokens(orml_tokens::Call::transfer {
				dest: EVA,
				currency_id: NATIVE_CURRENCY_ID,
				amount: schedule_transfer_amount.clone(),
			});

			let id = BlakeTwo256::hash_of(&schedule_call).as_fixed_bytes().to_vec();
			let init_locked_fund = 1000;
			// Fund the schedule call balance for the origin (ALICE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(ALICE), init_locked_fund));
			// Place the scheduled call for ALICE
			assert_ok!(Scheduler::schedule_call(
				Origin::signed(ALICE),
				5,
				Box::new(schedule_call.clone()),
				id.clone(),
				Some((5, 3)), /* Schedule the call every 5 blocks for 3 (2 repeates, 1
				               * first-time scheduled) times */
				1
			));

			// ALICE's native token balance before jumping to block 5 and executing the scheduled
			// transfer call
			let alice_balance_before_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// Jump to #block 5
			jump_to_block(5);
			// ALICE's native token balance after executing the scheduled transfer call
			let alice_balance_after_scheduled_call =
				Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID.clone());
			// The balance should be the same as the transfer call fails.
			assert_eq!(alice_balance_before_scheduled_call, alice_balance_after_scheduled_call);

			// Jump to #block 6 as the failed call is postponed one block forward
			jump_to_block(6);
			// Check if the locked funds are redeemable and is equal to the full amount deposited
			// initially
			assert_eq!(Scheduler::check_redeem_scheduled_call_fee(ALICE).unwrap(), true);
			// ALICE redeem her locked balance
			assert_ok!(Scheduler::redeem_schedule_fee(Origin::signed(ALICE)));
			assert_eq!(init_locked_fund, Scheduler::scheduled_locked_funds_balances(ALICE));
		})
}

#[should_panic]
#[test]
fn test_schedule_call_initial_deposit_charge_works() {
	// Should prevent placing schedule call if the
	// origin does not have enough locked funds
	// for the initial deposit
	// prepare a call
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			// Prepare the call
			let schedule_call = Call::Tokens(orml_tokens::Call::transfer {
				dest: BOB,
				currency_id: NATIVE_CURRENCY_ID,
				amount: 100000,
			});
			let init_locked_fund = 1000;
			// Fund the schedule call balance for the origin (ALICE)
			assert_ok!(Scheduler::fund_scheduled_call(Origin::signed(ALICE), init_locked_fund));
			let id = BlakeTwo256::hash_of(&schedule_call).as_fixed_bytes().to_vec();
			let call = Call::Scheduler(pallet::Call::schedule_call {
				when: 5,
				call: Box::new(schedule_call),
				id,
				maybe_periodic: None,
				priority: 1,
			});
			let len = call.encoded_size();
			let info = call.get_dispatch_info();
			charge_tx_fee(ALICE.clone(), &call, &info, len.clone());
		})
}

#[test]
fn test_cancel_schedule_call() {}
