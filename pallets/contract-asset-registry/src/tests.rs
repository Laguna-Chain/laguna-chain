use super::mock::*;
use crate::*;
use codec::Encode;
use primitives::AccountId;
use sp_core::Bytes;
use std::str::FromStr;

use frame_support::assert_ok;

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
	let deployed = evts
		.iter()
		.rev()
		.find_map(|rec| {
			if let Event::Contracts(pallet_contracts::Event::Instantiated {
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
fn test_total_supply() {
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (BOB, UNIT)])
		.build()
		.execute_with(|| {
			let init_amount = 1000_u64;
			let deployed = create_token(ALICE, "TKN", "TKN", init_amount);

			assert_ok!(ContractTokenRegistry::register_asset(
				Origin::root(),
				deployed.clone(),
				true
			));

			assert_eq!(
				ContractTokenRegistry::total_supply(deployed.clone()),
				Some(init_amount as u128)
			);

			assert_eq!(
				ContractTokenRegistry::balance_of(deployed.clone(), ALICE),
				Some(init_amount as u128)
			);

			assert_eq!(ContractTokenRegistry::balance_of(deployed.clone(), BOB), Some(0));

			assert_ok!(ContractTokenRegistry::transfer(
				deployed.clone(),
				ALICE,
				BOB,
				U256::from(init_amount / 10)
			));

			assert_eq!(
				ContractTokenRegistry::balance_of(deployed.clone(), BOB),
				Some(init_amount as u128 / 10)
			);

			// alice should have no allowance from bob to spend
			assert_eq!(ContractTokenRegistry::allowance(deployed.clone(), BOB, ALICE), Some(0));

			// alice should not be able to spend on be half of bob
			assert!(ContractTokenRegistry::transfer_from(
				deployed.clone(),
				ALICE,
				BOB,
				ALICE,
				U256::from(init_amount / 100)
			)
			.is_err());

			assert_eq!(
				ContractTokenRegistry::balance_of(deployed.clone(), BOB),
				Some(init_amount as u128 / 10)
			);

			// bob should be able to allow alice to spend
			assert_ok!(ContractTokenRegistry::approve(
				deployed.clone(),
				BOB,
				ALICE,
				U256::from(init_amount / 100)
			));

			// alice should have the correct allowance
			assert_eq!(
				ContractTokenRegistry::allowance(deployed.clone(), BOB, ALICE),
				Some(init_amount as u128 / 100)
			);

			assert_eq!(
				ContractTokenRegistry::balance_of(deployed.clone(), BOB),
				Some(init_amount as u128 / 10)
			);

			// alice should be able to spend the allowance
			assert_ok!(ContractTokenRegistry::transfer_from(
				deployed.clone(),
				ALICE,
				BOB,
				ALICE,
				U256::from(init_amount / 100)
			));

			assert_eq!(
				ContractTokenRegistry::balance_of(deployed, BOB),
				Some(init_amount as u128 * 90 / 1000)
			);
		});
}

#[test]
fn test_register() {
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (BOB, UNIT)])
		.sudo(ALICE)
		.build()
		.execute_with(|| {
			let deployed = create_token(ALICE, "ABC", "ABC", UNIT);

			assert_ok!(ContractTokenRegistry::register_asset(
				Origin::root(),
				deployed.clone(),
				true
			));

			assert_eq!(ContractTokenRegistry::get_registered(deployed), Some(true));
		});
}

#[test]
fn test_suspend() {
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (BOB, UNIT)])
		.sudo(ALICE)
		.build()
		.execute_with(|| {
			let deployed = create_token(ALICE, "ABC", "ABC", UNIT);

			assert_ok!(ContractTokenRegistry::register_asset(
				Origin::root(),
				deployed.clone(),
				true
			));

			assert_eq!(ContractTokenRegistry::get_registered(deployed.clone()), Some(true));

			assert_ok!(ContractTokenRegistry::suspend_asset(Origin::root(), deployed.clone(),));

			assert_eq!(ContractTokenRegistry::get_registered(deployed), Some(false));
		});
}

#[test]
fn test_unregister() {
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (BOB, UNIT)])
		.sudo(ALICE)
		.build()
		.execute_with(|| {
			let deployed = create_token(ALICE, "ABC", "ABC", UNIT);

			assert_ok!(ContractTokenRegistry::register_asset(
				Origin::root(),
				deployed.clone(),
				true
			));

			assert_eq!(ContractTokenRegistry::get_registered(deployed.clone()), Some(true));

			assert_ok!(ContractTokenRegistry::unregister_asset(Origin::root(), deployed.clone(),));

			assert_eq!(ContractTokenRegistry::get_registered(deployed), None);
		});
}
