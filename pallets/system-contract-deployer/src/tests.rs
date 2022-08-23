use super::*;
use frame_support::{assert_err, assert_ok, error, sp_runtime};
use mock::{Call, Event, ExtBuilder, Origin, Sudo, SudoContracts, System, Test, ALICE, UNIT};
use sp_core::Bytes;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use std::str::FromStr;

const MAX_GAS: u64 = 200_000_000_000;

#[test]
fn test_fixed_address() {
	let deploying_key = <Test as crate::Config>::PalletId::get()
		.try_into_account()
		.expect("Invalid PalletId");
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (deploying_key, UNIT)])
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

			pub type SudoContractsCall = crate::Call<Test>;
			let call = Box::new(Call::SudoContracts(SudoContractsCall::instantiate_with_code {
				value: 0,
				gas_limit: MAX_GAS,
				storage_deposit_limit: None,
				code: blob,
				data: sel_constructor,
				destined_address: Some([0x11; 32]),
			}));

			assert_ok!(Sudo::sudo(Origin::signed(ALICE), call));

			let evts = System::events();

			let deployed_addr = evts
				.iter()
				.rev()
				.find_map(|r| {
					if let Event::SudoContracts(crate::Event::Created(contract)) = &r.event {
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
fn test_sequential_address_generation() {
	let deploying_key = <Test as crate::Config>::PalletId::get()
		.try_into_account()
		.expect("Invalid PalletId");
	ExtBuilder::default()
		.balances(vec![(ALICE, UNIT), (deploying_key, UNIT)])
		.sudo(ALICE)
		.build()
		.execute_with(|| {
			// 1. Upload code
			let blob = std::fs::read(
				"../../runtime/integration-tests/contracts-data/ink/basic/dist/basic.wasm",
			)
			.expect("cound not find wasm blob");

			let sel_constructor = Bytes::from_str("0xed4b9d1b")
				.map(|v| v.to_vec())
				.expect("unable to parse hex string");

			let ch = pallet_contracts::Pallet::<Test>::bare_upload_code(ALICE, blob, None)
				.unwrap()
				.code_hash;

			let deploy_contract = || {
				assert_ok!(crate::Pallet::<Test>::instantiate(
					Origin::root(),
					0,
					MAX_GAS,
					None,
					ch,
					sel_constructor.clone(),
					None,
				));

				let evts = System::events();

				let addr = evts
					.iter()
					.rev()
					.find_map(|r| {
						if let Event::SudoContracts(crate::Event::Created(contract)) = &r.event {
							Some(contract)
						} else {
							None
						}
					})
					.expect("unable to find contract");

				System::reset_events();
				addr.clone()
			};

			// 2. Deploy multiple instances
			let addr1 = deploy_contract();
			let addr2 = deploy_contract();

			// 3. Verify contract addresses
			let expected_addr1 = AccountId32::from_str(&format!("{:064x}", 1)).unwrap();
			let expected_addr2 = AccountId32::from_str(&format!("{:064x}", 2)).unwrap();

			assert_eq!(addr1, expected_addr1);
			assert_eq!(addr2, expected_addr2);
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

		assert_err!(
			SudoContracts::instantiate_with_code(
				Origin::signed(ALICE),
				0,
				MAX_GAS,
				None,
				blob,
				sel_constructor,
				Some([0x11; 32]),
			),
			error::BadOrigin
		);
	})
}
