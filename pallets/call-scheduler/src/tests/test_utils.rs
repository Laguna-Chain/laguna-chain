use super::*;
use crate::mock::{Call, Event, *};
use core::str::FromStr;
use frame_support::assert_ok;
use primitives::AccountId;
use sp_core::{Bytes, U256};

pub fn create_token<T>(
	owner: AccountId,
	tkn_name: &str,
	tkn_symbol: &str,
	init_amount: T,
) -> AccountId
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

pub fn jump_to_block(num: u32) {
	while System::block_number() < num {
		Scheduler::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Scheduler::on_initialize(System::block_number());
	}
}

pub fn charge_tx_fee(account: AccountId, call: &Call, info: &DispatchInfo, len: usize) {
	ChargeTransactionPayment::<Runtime>::from(0)
		.pre_dispatch(&ALICE, &call, &info, len)
		.expect("should pass");
}
