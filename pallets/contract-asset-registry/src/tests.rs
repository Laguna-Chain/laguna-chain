use super::mock::*;
use crate::*;
use codec::Encode;
use sp_core::Bytes;
use std::str::FromStr;

use frame_support::assert_ok;

#[test]
fn test_total_supply() {
	ExtBuilder::default().balances(vec![(ALICE, UNIT)]).build().execute_with(|| {
		let blob = std::fs::read(
			"../../runtime/integration-tests/contracts-data/solidity/token/dist/DemoToken.wasm",
		)
		.expect("unable to read contract");

		let mut sel_constuctor = Bytes::from_str("0x835a15cb")
			.map(|v| v.to_vec())
			.expect("unable to parse selector");

		let init_amount = 100_000_000_u64;

		sel_constuctor.append(&mut "FAKE_TOKEN".encode());
		sel_constuctor.append(&mut "TKN".encode());
		sel_constuctor.append(&mut U256::from(init_amount).encode());

		assert_ok!(Contracts::instantiate_with_code(
			Origin::signed(ALICE),
			0,
			MAX_GAS,
			None,
			blob,
			sel_constuctor,
			vec![]
		));

		let evts = System::events();
		let deployed = evts
			.iter()
			.rev()
			.filter_map(|rec| {
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
			.next()
			.expect("unable to find deployed contract");

		// freshly create token have no supply
		assert_eq!(
			ContractTokenRegistry::total_supply(deployed.clone()),
			Some(init_amount as u128)
		);
	});
}
