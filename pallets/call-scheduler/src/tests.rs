//! Unit test for call-scheduler
use super::*;
use crate::mock::{Call, Event, *};
use core::str::FromStr;
use traits::currencies::TokenAccess;

use frame_support::{assert_ok, dispatch::DispatchInfo};
use pallet_transaction_payment::ChargeTransactionPayment;
use primitives::{AccountId, CurrencyId};
use sp_core::{Bytes, U256};
use sp_runtime::{traits::SignedExtension, AccountId32};

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
			assert_ok!(FluentFee::set_preferred_fee_asset(
				Origin::signed(ALICE),
				CURRENCY_ID.clone()
			));
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

			eprintln!("########################## fee: {} ########################", fee);

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
