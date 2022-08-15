use crate::{adapters::CurrencyAdapter, mock::{Event as MockEvents,*}, AccountIdOf};
use codec::Encode;
use frame_support::{
	assert_err, assert_ok, parameter_types,
	traits::{fungible, fungibles},
};

use core::cell::Cell;
use orml_traits::{parameter_type_with_key, BasicCurrency, MultiCurrency};
use primitives::{AccountId, CurrencyId, TokenId};
use sp_core::{Bytes, U256};
use std::{str::FromStr, sync::Mutex};
use traits::currencies::TokenAccess;

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
			if let MockEvents::Contracts(pallet_contracts::Event::Instantiated {
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

#[test]
fn test_adapter_inspect() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			parameter_types! {
				pub const TargetAssetId: CurrencyId= CurrencyId::NativeToken(TokenId::Laguna);
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, TargetAssetId>;

			assert_eq!(
				<NativeTokenAdapter as fungible::Inspect<_>>::total_issuance(),
				<Currencies as fungibles::Inspect<_>>::total_issuance(TargetAssetId::get())
			);

			assert_eq!(
				<NativeTokenAdapter as fungible::Inspect<_>>::minimum_balance(),
				<Currencies as fungibles::Inspect<_>>::minimum_balance(TargetAssetId::get())
			);

			assert_eq!(
				<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE),
				<Currencies as fungibles::Inspect<_>>::balance(TargetAssetId::get(), &ALICE)
			);

			assert_eq!(
				<NativeTokenAdapter as fungible::Inspect<_>>::reducible_balance(&ALICE, true),
				<Currencies as fungibles::Inspect<_>>::reducible_balance(
					TargetAssetId::get(),
					&ALICE,
					true
				)
			);

			assert_eq!(
				<NativeTokenAdapter as fungible::Inspect<_>>::can_deposit(&ALICE, 1000, true),
				<Currencies as fungibles::Inspect<_>>::can_deposit(
					TargetAssetId::get(),
					&ALICE,
					1000,
					true
				)
			);

			assert_eq!(
				<NativeTokenAdapter as fungible::Inspect<_>>::can_withdraw(&ALICE, 1000),
				<Currencies as fungibles::Inspect<_>>::can_withdraw(
					TargetAssetId::get(),
					&ALICE,
					1000,
				)
			);
		});
}

#[test]
fn test_adapter_mutate() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			parameter_types! {
				pub const TargetAssetId: CurrencyId= CurrencyId::NativeToken(TokenId::Laguna);
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, TargetAssetId>;

			assert_ok!(<NativeTokenAdapter as fungible::Mutate<_>>::mint_into(&ALICE, 1000));
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE), UNIT + 1000);

			assert_ok!(<Currencies as fungibles::Mutate<_>>::mint_into(
				TargetAssetId::get(),
				&ALICE,
				1000
			));

			assert_eq!(
				<Currencies as fungibles::Inspect<_>>::balance(TargetAssetId::get(), &ALICE),
				UNIT + 2000
			);

			assert_ok!(<NativeTokenAdapter as fungible::Mutate<_>>::burn_from(&ALICE, 1000));
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE), UNIT + 1000);
		});
}

#[test]
fn test_adapter_transfer() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			parameter_types! {
				pub const TargetAssetId: CurrencyId= CurrencyId::NativeToken(TokenId::Laguna);
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, TargetAssetId>;

			assert_ok!(<NativeTokenAdapter as fungible::Transfer<_>>::transfer(
				&ALICE, &BOB, 1000, true
			));
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE), UNIT - 1000);
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&BOB), UNIT + 1000);
		});
}

#[test]
fn test_adapter_unbalanced() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			parameter_types! {
				pub const TargetAssetId: CurrencyId= CurrencyId::NativeToken(TokenId::Laguna);
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, TargetAssetId>;

			assert_ok!(<NativeTokenAdapter as fungible::Unbalanced<_>>::set_balance(&ALICE, 0));
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE), 0);

			<NativeTokenAdapter as fungible::Unbalanced<_>>::set_total_issuance(0);
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::total_issuance(), 0);
		});
}

#[test]
fn test_adapter_inspect_hold() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			parameter_types! {
				pub const TargetAssetId: CurrencyId= CurrencyId::NativeToken(TokenId::Laguna);
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, TargetAssetId>;

			assert_eq!(
				<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&ALICE),
				0
			);

			assert!(<NativeTokenAdapter as fungible::InspectHold<_>>::can_hold(&ALICE, UNIT));
			assert!(!<NativeTokenAdapter as fungible::InspectHold<_>>::can_hold(&ALICE, UNIT + 1));
		});
}

#[test]
fn test_adapter_mutate_hold() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			parameter_types! {
				pub const TargetAssetId: CurrencyId= CurrencyId::NativeToken(TokenId::Laguna);
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, TargetAssetId>;

			assert_ok!(<NativeTokenAdapter as fungible::MutateHold<_>>::hold(&ALICE, UNIT));
			assert_eq!(
				<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&ALICE),
				UNIT
			);
			assert!(!<NativeTokenAdapter as fungible::InspectHold<_>>::can_hold(&ALICE, UNIT));

			assert_ok!(<NativeTokenAdapter as fungible::MutateHold<_>>::release(
				&ALICE, UNIT, true
			));
			assert_eq!(
				<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&ALICE),
				0
			);
			assert!(<NativeTokenAdapter as fungible::InspectHold<_>>::can_hold(&ALICE, UNIT));

			assert_ok!(<NativeTokenAdapter as fungible::MutateHold<_>>::hold(&ALICE, UNIT));
			assert_eq!(
				<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&ALICE),
				UNIT
			);
			assert_ok!(<NativeTokenAdapter as fungible::MutateHold<_>>::transfer_held(
				&ALICE, &BOB, UNIT, true, false
			));
			assert_eq!(
				<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&ALICE),
				0
			);
			assert_eq!(<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&BOB), 0);
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&BOB), 2 * UNIT);

			assert_ok!(<NativeTokenAdapter as fungible::MutateHold<_>>::hold(&BOB, UNIT));
			assert_ok!(<NativeTokenAdapter as fungible::MutateHold<_>>::transfer_held(
				&BOB, &ALICE, UNIT, true, true
			));
			assert_eq!(<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&BOB), 0);
			assert_eq!(
				<NativeTokenAdapter as fungible::InspectHold<_>>::balance_on_hold(&ALICE),
				UNIT
			);
		});
}

use once_cell::sync::Lazy;
static CID: Lazy<Mutex<CurrencyId>> = Lazy::new(|| Mutex::new(CurrencyId::Erc20([0_u8; 32])));

#[test]
fn test_adapter_erc20() {
	let init_amount = UNIT;
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), init_amount),
			(BOB, NativeCurrencyId::get(), init_amount),
		])
		.build()
		.execute_with(|| {
			let deployed = create_token(ALICE, "ABC", "ABC", UNIT);
			assert_ok!(ContractTokenRegistry::register_asset(
				Origin::root(),
				deployed.clone(),
				true
			));
			let cid = CurrencyId::Erc20(*deployed.as_ref());

			{
				let mut handle = CID.lock().unwrap();
				*handle = cid;
			}

			parameter_types! {
				pub AssetId: CurrencyId = {
				let  out = {
					let handle = CID.lock().unwrap();
					handle.clone()
				};
				out
				};
			}

			type NativeTokenAdapter = CurrencyAdapter<Runtime, AssetId>;
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE), UNIT);
			assert_ok!(<NativeTokenAdapter as fungible::Transfer<_>>::transfer(
				&ALICE, &BOB, 1000, true
			));

			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&ALICE), UNIT - 1000);
			assert_eq!(<NativeTokenAdapter as fungible::Inspect<_>>::balance(&BOB), 1000);
		});
}
