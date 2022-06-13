//! Unit test for the fluent-fee pallet

use super::*;

use frame_support::{assert_ok, dispatch::DispatchInfo};
use mock::*;
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::traits::SignedExtension;

#[test]
fn test_staking_asset() {
	ExtBuilder::default()
		.balances(vec![(ALICE, FEE_TOKEN_ID, 1_000_000_000), (BOB, FEE_TOKEN_ID, 1_000_000_000)])
		.build()
		.execute_with(|| {
			// assert_eq!(Currencies::free_balance(FEE_TOKEN_ID, &ALICE), 1_000_000_000);
			// assert_eq!(Currencies::free_balance(FEE_TOKEN_ID, &BOB), 1_000_000_000);
			assert_ok!(FluentFee::stake(Origin::signed(ALICE), FEE_TOKEN_ID, 390_000_000));
			assert_eq!(FluentFee::total_staked(FEE_TOKEN_ID), 390_000_000);
		})
}

#[test]
fn test_listing_asset_ok() {
	ExtBuilder::default()
		.balances(vec![(ALICE, FEE_TOKEN_ID, 1_000_000_000), (BOB, FEE_TOKEN_ID, 1_000_000_000)])
		.build()
		.execute_with(|| {
			// 700k/2000k = 35% of the total supply of FluentFee staked
			assert_ok!(FluentFee::stake(Origin::signed(ALICE), FEE_TOKEN_ID, 700_000_000));
			assert_ok!(FluentFee::listing_asset(Origin::signed(ALICE), FEE_TOKEN_ID));
		})
}

#[test]
#[should_panic]
fn test_listing_asset_not_ok() {
	ExtBuilder::default()
		.balances(vec![(ALICE, FEE_TOKEN_ID, 1_000_000_000), (BOB, FEE_TOKEN_ID, 1_000_000_000)])
		.build()
		.execute_with(|| {
			// 29% of the total FluentFee supply staked
			assert_ok!(FluentFee::stake(Origin::signed(ALICE), FEE_TOKEN_ID, 580_000_000));
			assert_ok!(FluentFee::listing_asset(Origin::signed(ALICE), FEE_TOKEN_ID));
		})
}

#[test]
fn test_accepted_assets() {
	ExtBuilder::default()
		.balances(vec![(ALICE, FEE_TOKEN_ID, 1_000_000_000), (BOB, FEE_TOKEN_ID, 1_000_000_000)])
		.build()
		.execute_with(|| {
			// 29% of the total FluentFee supply staked
			assert_ok!(FluentFee::stake(Origin::signed(ALICE), FEE_TOKEN_ID, 700_000_000));
			assert_ok!(FluentFee::listing_asset(Origin::signed(ALICE), FEE_TOKEN_ID));
			assert_ok!(FluentFee::check_accepted_asset(Origin::signed(ALICE), FEE_TOKEN_ID));
		})
}

#[test]
fn test_set_priority_fee_asset() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, FEE_TOKEN_ID, 1_000_000_000),
			(ALICE, NATIVE_CURRENCY_ID, 1_000_000_000),
		])
		.build()
		.execute_with(|| {
			// should fail as FEE_TOKEN_ID should first be listed as a valid fee source
			assert_ok!(FluentFee::set_fee_source_priority(Origin::signed(ALICE), FEE_TOKEN_ID));
			assert_eq!(FluentFee::account_fee_source_priority(ALICE), Some(FEE_TOKEN_ID));
		})
}
