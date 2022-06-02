use crate::{mock::*, AccountIdOf};
use codec::Encode;
use frame_support::{
	assert_err, assert_ok,
	traits::{fungible, fungibles},
};
use orml_traits::{BasicCurrency, MultiCurrency};
use pallet_contract_asset_registry::TokenAccess;
use primitives::{AccountId, CurrencyId};
use sp_core::{Bytes, U256};
use std::str::FromStr;

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
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Currencies as fungibles::Inspect<AccountIdOf<Runtime>>>::balance(
					NativeCurrencyId::get(),
					&ALICE
				),
				init_amount
			);
		});
}

#[test]
fn test_total_supply_erc20() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			let deployed = create_token(ALICE, "ABC", "ABC", UNIT);
			let cid = CurrencyId::Erc20(*deployed.as_ref());
			assert_eq!(
				<Currencies as MultiCurrency<AccountIdOf<Runtime>>>::free_balance(cid, &ALICE),
				0
			);

			assert_ok!(ContractTokenRegistry::register_asset(
				Origin::root(),
				deployed.clone(),
				true
			));

			assert_eq!(ContractTokenRegistry::balance_of(deployed.clone(), ALICE), Some(UNIT));
			assert_eq!(
				<Currencies as MultiCurrency<AccountIdOf<Runtime>>>::free_balance(cid, &ALICE),
				UNIT
			);

			assert_err!(
				<Currencies as MultiCurrency<AccountIdOf<Runtime>>>::deposit(cid, &ALICE, UNIT),
				crate::Error::<Runtime>::InvalidContractOperation
			);

			assert_err!(
				<Currencies as MultiCurrency<AccountIdOf<Runtime>>>::withdraw(cid, &ALICE, UNIT),
				crate::Error::<Runtime>::InvalidContractOperation
			);

			assert_ok!(<Currencies as MultiCurrency<AccountIdOf<Runtime>>>::transfer(
				cid, &ALICE, &BOB, UNIT
			),);

			assert_eq!(ContractTokenRegistry::balance_of(deployed.clone(), ALICE), Some(0));
			assert_eq!(ContractTokenRegistry::balance_of(deployed.clone(), BOB), Some(UNIT));
		});
}
