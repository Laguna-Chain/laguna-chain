//! Unit test for the fluent-fee pallet

use core::str::FromStr;

use super::mock::*;
use crate::*;

use frame_support::{assert_ok, dispatch::DispatchInfo};
use pallet_transaction_payment::ChargeTransactionPayment;
use primitives::AccountId;
use sp_core::{Bytes, U256};
use sp_runtime::{traits::SignedExtension, AccountId32};

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

fn create_token<T>(owner: AccountId, tkn_name: &str, tkn_symbol: &str, init_amount: T) -> AccountId
where
	U256: From<T>,
{
	let blob = std::fs::read(
		"../../runtime/integration-tests/contracts-data/solidity/token/dist/DemoToken.wasm",
	)
	.expect("unable to read contract");

	let mut sel_constuctor = Bytes::from_str("0x835a15cb")
		.map(|v| v.to_vec())
		.expect("unable to parse selector");

	sel_constuctor.append(&mut tkn_name.encode());
	sel_constuctor.append(&mut tkn_symbol.encode());
	sel_constuctor.append(&mut U256::from(init_amount).encode());

	assert_ok!(Contracts::instantiate_with_code(
		Origin::signed(owner),
		0,
		MaxGas::get(),
		None, /* if not specified, it's allowed to charge the max amount of free balance of the
		       * creator */
		blob,
		sel_constuctor,
		vec![]
	));

	let evts = System::events();
	// dbg!(evts.clone());
	let deployed = evts
		.iter()
		.rev()
		.find_map(|rec| {
			if let mock::Event::Contracts(pallet_contracts::Event::Instantiated {
				deployer: _,
				contract,
			}) = &rec.event
			{
				Some(contract)
			} else {
				None
			}
		})
		.expect("unable to find deployed contract");

	deployed.clone()
}

#[test]
fn test_set_priority_fee_asset() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 1000000000000000000000000000000000),
			(BOB, NATIVE_CURRENCY_ID, 10000000000000000000000000000000000),
		])
		.build()
		.execute_with(|| {
			let init_amount = 1000000000000000000000_u128;
			let deployed = create_token(ALICE, "TKN", "TKN", init_amount);
			// let deployed: AccountId32 = AccountId32::from([1u8; 32]);

			assert_ok!(ContractAssets::register_asset(Origin::root(), deployed.clone(), true));
			// List the ERC20 in the fluent fee pallet
			assert_ok!(FluentFee::listing_asset(
				Origin::signed(ALICE),
				CurrencyId::Erc20(deployed.clone().into())
			));
			// set the ERC20 as ALICE's prioritized gas fee source
			assert_ok!(FluentFee::set_fee_source_priority(
				Origin::signed(ALICE),
				CurrencyId::Erc20(deployed.clone().into())
			));
			// check if the ERC20 is being accepted
			assert_ok!(FluentFee::check_accepted_asset(
				Origin::signed(ALICE),
				CurrencyId::Erc20(deployed.clone().into())
			));
			// make some arbitray calls and use the erc20 token as the fee source
			assert_ok!(FluentFee::prepay_fees(
				Origin::signed(ALICE),
				CurrencyId::NativeToken(TokenId::Laguna),
				1000
			));
		})
}
