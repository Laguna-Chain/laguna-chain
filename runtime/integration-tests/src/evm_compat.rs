#[cfg(test)]
mod tests {

	use frame_support::assert_ok;
	use hydro_runtime::{constants::HYDROS, Call, Event, Evm, Origin, Runtime, Sudo, System};
	use pallet_evm::{AddressMapping, Runner};
	use precompile_utils::{EvmDataReader, EvmDataWriter};
	use sp_core::{H160, U256};
	use std::process;

	use sp_core::bytes::from_hex;

	use crate::{ExtBuilder, ALICE, NATIVE_CURRENCY_ID};

	fn prepare_smart_contract(rel_contract_path: &str) -> String {
		let contracts_dir = std::option_env!("CONTRACTS_DIR").expect("path not specified");

		// compile the smart contract using hardhat
		process::Command::new("bash")
			.current_dir(contracts_dir)
			.args(["-c", "npx hardhat compile"])
			.output()
			.expect("unable to compile using hardhat");

		let artifact_path = format!("{}/artifacts/contracts/{}", contracts_dir, rel_contract_path);

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

		return bytecode
	}

	#[test]
	fn check_compiler_result() {
		// TODO: check result across different solc version version

		// bytecode generated from remix with solidity compiler version 0.8.11
		let expected_bytecode = "0x608060405234801561001057600080fd5b506101da806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c8063165c4a1614610030575b600080fd5b61004a600480360381019061004591906100b1565b610060565b6040516100579190610100565b60405180910390f35b6000818361006e919061014a565b905092915050565b600080fd5b6000819050919050565b61008e8161007b565b811461009957600080fd5b50565b6000813590506100ab81610085565b92915050565b600080604083850312156100c8576100c7610076565b5b60006100d68582860161009c565b92505060206100e78582860161009c565b9150509250929050565b6100fa8161007b565b82525050565b600060208201905061011560008301846100f1565b92915050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b60006101558261007b565b91506101608361007b565b9250817fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff04831182151516156101995761019861011b565b5b82820290509291505056fea2646970667358221220aa0cede4656b6aa20ccc3a47a3362870ed8939786a4a2174fa528b498b76f37f64736f6c634300080b0033";

		let generated_bytecode = prepare_smart_contract("Multiply.sol/Multiply.json");

		assert_eq!(expected_bytecode, &generated_bytecode);
	}

	#[test]
	fn deploy_simple_contract() {
		let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
		let mapped_account =
			<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

		ExtBuilder::default()
			.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.sudo(mapped_account.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
			.build()
			.execute_with(|| {
				let contract = from_hex(&prepare_smart_contract("Multiply.sol/Multiply.json"))
					.expect("unable to parse hex string");

				// prepare the evm call to be submitted by the Sudo pallet
				let evm_call = Call::Evm(pallet_evm::Call::create {
					source: evm_address,
					init: contract,
					value: 0_u64.into(),
					gas_limit: u64::MAX,
					max_fee_per_gas: 0_u64.into(),
					max_priority_fee_per_gas: None,
					nonce: None,
					access_list: vec![],
				});

				// sudo should be able to call evm with sudo
				let rs = Sudo::sudo(Origin::signed(mapped_account.clone()), Box::new(evm_call));
				assert_ok!(&rs);

				let events = System::events();
				assert!(events.len() != 0);

				// check if the last evm event is of expected type
				let evm_evts =
					events
						.iter()
						.map(|record| &record.event)
						.filter_map(|event| {
							if let Event::Evm(evm_evt) = event {
								Some(evm_evt)
							} else {
								None
							}
						})
						.collect::<Vec<_>>();

				let evt = evm_evts.last().unwrap();

				#[precompile_utils::generate_function_selector]
				#[derive(Debug, PartialEq)]
				enum Action {
					Multiply = "multiply(uint256,uint256)",
				}

				match &evt {
					// extract deployed address from pallet_evm::Event::Created
					pallet_evm::Event::Created(deployed_address) => {
						let input = EvmDataWriter::new_with_selector(Action::Multiply)
							.write(U256::from(2_u64))
							.write(U256::from(3_u64))
							.build();

						// raw evm execution using T::Runner so we can inspect output
						let rs = <Runtime as pallet_evm::Config>::Runner::call(
							evm_address,
							*deployed_address,
							input,
							0_u64.into(),
							u64::MAX,
							None,
							None,
							None,
							vec![],
							<Runtime as pallet_evm::Config>::config(),
						);

						assert_ok!(&rs);

						// extract evm output value from raw bytes
						let info = rs.unwrap();
						let value: Result<U256, _> = EvmDataReader::new(&info.value).read();
						assert_ok!(&value);
						let value = value.unwrap();

						assert_eq!(value, (2_u64 * 3).into());
					},
					_ => {
						panic!("shouldn't be of any other type");
					},
				}
			});
	}

	#[test]
	fn compile_native_lib() {
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

				let contract =
					from_hex(&prepare_smart_contract("NativeCurrency.sol/NativeCurrency.json"))
						.expect("unable to parse smart contract");

				// prepare the evm call to be submitted by the Sudo pallet
				let evm_call = Call::Evm(pallet_evm::Call::create {
					source: evm_address,
					init: contract,
					value: 0_u64.into(),
					gas_limit: u64::MAX,
					max_fee_per_gas: 0_u64.into(),
					max_priority_fee_per_gas: None,
					nonce: None,
					access_list: vec![],
				});

				// sudo should be able to call evm with sudo
				let rs = Sudo::sudo(Origin::signed(mapped_account.clone()), Box::new(evm_call));
				assert_ok!(&rs);

				let events = System::events();
				assert!(events.len() != 0);

				let evm_evts =
					events
						.iter()
						.map(|record| &record.event)
						.filter_map(|event| {
							if let Event::Evm(evm_evt) = event {
								Some(evm_evt)
							} else {
								None
							}
						})
						.collect::<Vec<_>>();

				let evt = evm_evts.last().unwrap();

				#[precompile_utils::generate_function_selector]
				#[derive(Debug, PartialEq)]
				enum Action {
					BalanceOf = "balanceOf(address)",
				}

				match &evt {
					// extract deployed address from pallet_evm::Event::Created
					pallet_evm::Event::Created(deployed_address) => {
						let input = EvmDataWriter::new_with_selector(Action::BalanceOf)
							.write(precompile_utils::Address(evm_address))
							.build();

						// raw evm execution using T::Runner so we can inspect output
						let rs = <Runtime as pallet_evm::Config>::Runner::call(
							evm_address,
							*deployed_address,
							input,
							0_u64.into(),
							u64::MAX,
							None,
							None,
							None,
							vec![],
							<Runtime as pallet_evm::Config>::config(),
						);

						assert_ok!(&rs);

						// extract evm output value from raw bytes
						let info = rs.unwrap();
						dbg!(&info);
						let value: Result<U256, _> = EvmDataReader::new(&info.value).read();
						assert_ok!(&value);
						let value = value.unwrap();

						assert_eq!(value, (10 * HYDROS).into());
					},
					_ => {
						panic!("shouldn't be of any other type");
					},
				}
			});
	}
}
