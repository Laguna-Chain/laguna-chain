use super::*;
use frame_support::assert_ok;
use mock::{
	Call, Event, ExtBuilder, Origin, Sudo, SudoContract, System, Test, ALICE, BURN_ADDR, UNIT,
};
use sp_core::Bytes;
use sp_runtime::AccountId32;
use std::str::FromStr;

const MAX_GAS: u64 = 200_000_000_000;

#[test]
fn test_fixed_address() {
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (BURN_ADDR, UNIT)])
		.sudo(ALICE)
		.build()
		.execute_with(|| {
			let blob = std::fs::read(
				"../../runtime/integration-tests/contracts-data/ink/basic/dist/basic.wasm",
			)
			.expect("cound not find wasm blob");

			let sel_constructor = Bytes::from_str("0xed4b9d1b")
				.map(|v| v.to_vec())
				.expect("unable to parse hex string");

			pub type SudoContractCall = crate::Call<Test>;
			let call = Box::new(Call::SudoContract(SudoContractCall::instantiate_with_code {
				value: 0,
				gas_limit: MAX_GAS,
				storage_deposit_limit: None,
				code: blob,
				data: sel_constructor,
				salt: vec![0x11; 32],
			}));

			assert_ok!(Sudo::sudo(Origin::signed(ALICE), call));

			let evts = System::events();

			let deployed_addr = evts
				.iter()
				.rev()
				.find_map(|r| {
					if let Event::Contracts(pallet_contracts::Event::Instantiated {
						deployer,
						contract,
					}) = &r.event
					{
						Some(contract)
					} else {
						None
					}
				})
				.expect("unable to find contract");

			assert_eq!(deployed_addr, &AccountId32::from([0x11; 32]));
		})
}

#[test]
fn test_only_root_access() {
	ExtBuilder::default().balances(vec![(ALICE, UNIT)]).build().execute_with(|| {
		let blob = std::fs::read(
			"../../runtime/integration-tests/contracts-data/ink/basic/dist/basic.wasm",
		)
		.expect("cound not find wasm blob");

		let sel_constructor = Bytes::from_str("0xed4b9d1b")
			.map(|v| v.to_vec())
			.expect("unable to parse hex string");

		assert!(SudoContract::instantiate_with_code(
			Origin::signed(ALICE),
			0,
			MAX_GAS,
			None,
			blob,
			sel_constructor,
			vec![0x11; 32],
		)
		.is_err());
	})
}
