#[cfg(test)]
mod tests {

	use frame_support::assert_ok;
	use hydro_runtime::{constants::HYDROS, Call, Event, Evm, Origin, Runtime, Sudo, System};
	use pallet_evm::AddressMapping;
	use sp_core::H160;
	use std::process;

	use crate::{ExtBuilder, ALICE, NATIVE_CURRENCY_ID};

	#[test]
	fn compile_native_lib() {
		// load the smart contract directory from env
		let contracts_dir = std::option_env!("CONTRACTS_DIR").expect("path not specified");

		// compile the smart contract using hardhat
		process::Command::new("bash")
			.current_dir(contracts_dir)
			.args(["-c", "npx hardhat compile"])
			.output()
			.expect("unable to compile using hardhat");

		let artifact_path =
			format!("{}/artifacts/contracts/Greeter.sol/Greeter.json", contracts_dir);

		// compile the smart contract using hardhat
		let bytecode_output = process::Command::new("bash")
			.current_dir(contracts_dir)
			.args(["-c", format!("jql {} {} -r", r#"'"bytecode"'"#, artifact_path).as_str()])
			.output()
			.expect("unable to parse bytecode using jql");

		// extract bytecode from jql output
		let bytecode = bytecode_output
			.stdout
			.iter()
			.map(|c| *c as char)
			.collect::<String>()
			.replace('\n', "");

		// genrate AccountId from evm addres
		let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
		let mapped_account =
			<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

		ExtBuilder::default()
			.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.sudo(mapped_account.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
			.build()
			.execute_with(|| {
				// check evm_account storages
				assert_eq!(pallet_evm::AccountCodes::<Runtime>::iter().count(), 0);

				// prepare the evm call to be submitted by the Sudo pallet
				let evm_call = Call::Evm(pallet_evm::Call::create {
					source: evm_address,
					init: bytecode.as_bytes().to_vec(),
					value: 0_u64.into(),
					gas_limit: u64::MAX,
					max_fee_per_gas: 0_u64.into(),
					max_priority_fee_per_gas: None,
					nonce: None,
					access_list: vec![],
				});

				// sudo should be able to call evm with sudo
				let rs = Sudo::sudo(Origin::signed(mapped_account), Box::new(evm_call));
				assert_ok!(&rs);

				let events = System::events();
				assert!(events.len() != 0);

				let evm_evts = events
					.iter()
					.map(|record| record.event.clone())
					.filter_map(|event| {
						if let Event::Evm(evm_evt) = event {
							Some(evm_evt.clone())
						} else {
							None
						}
					})
					.collect::<Vec<_>>();

				let evt = &evm_evts.last().unwrap();

				match evt {
					pallet_evm::Event::Created(deployed_address) => todo!(),
					pallet_evm::Event::CreatedFailed(attempted_address) => {
						dbg!(attempted_address);
					},
					_ => {
						panic!("shouldn't be of any other type");
					},
				}
			});
	}
}
