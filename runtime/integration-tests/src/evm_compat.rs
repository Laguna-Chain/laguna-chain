#[cfg(test)]
mod tests {

	use frame_support::assert_ok;
	use hydro_runtime::{
		constants::HYDROS, Call, Currencies, Event, Evm, Origin, Runtime, Sudo, System,
	};
	use orml_traits::MultiCurrency;
	use pallet_evm::{AddressMapping, Runner};
	use precompile_utils::{Address, EvmDataReader, EvmDataWriter};
	use sp_core::{H160, U256};
	use std::process;

	use sp_core::bytes::from_hex;

	use crate::{ExtBuilder, ALICE, NATIVE_CURRENCY_ID};

	pub mod erc20;

	/// contract_name: the solidity source file
	/// target_name: the struct/interface/library being called
	pub fn prepare_smart_contract(contract_name: &str, target_name: &str) -> String {
		let contracts_dir = std::option_env!("CONTRACTS_DIR").expect("path not specified");

		// compile the smart contract using hardhat
		let compile_output = process::Command::new("bash")
			.current_dir(contracts_dir)
			.args(["-c", "npx hardhat compile"])
			.output()
			.expect("unable to compile using hardhat");

		assert!(compile_output.stderr.len() == 0);

		let artifact_path = format!(
			"{}/artifacts/contracts/{}.sol/{}.json",
			contracts_dir, contract_name, target_name
		);

		// compile the smart contract using hardhat
		let bytecode_output = process::Command::new("bash")
			.current_dir(contracts_dir)
			.args(["-c", format!("jql {} {} -r", r#"'"bytecode"'"#, artifact_path).as_str()])
			.output()
			.expect("unable to parse bytecode using jql");

		// compilation should not fail
		assert!(bytecode_output.stderr.len() == 0);

		let bytecode = String::from_utf8(bytecode_output.stdout)
			.map(|s| s.strip_suffix("\n").unwrap_or_default().to_string())
			.unwrap_or_default();

		return bytecode
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
				let contract = from_hex(&prepare_smart_contract("Basic", "Multiply"))
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
	fn consume_platform_library() {
		let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
		let mapped_account =
			<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

		ExtBuilder::default()
			.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.sudo(mapped_account.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
			.build()
			.execute_with(|| {
				let contract = from_hex(&prepare_smart_contract("PlatformConsumer", "Native"))
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
				enum NativeAction {
					Name = "name()",
					BalanceOf = "balanceOf(address)",
				}

				match &evt {
					// extract deployed address from pallet_evm::Event::Created
					pallet_evm::Event::Created(deployed_address) => {
						let input = EvmDataWriter::new_with_selector(NativeAction::Name).build();
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
						let info = rs.unwrap();

						// extract evm output value from raw bytes
						let value: Result<precompile_utils::Bytes, _> =
							EvmDataReader::new(&info.value).read();
						assert_ok!(&value);

						let value = value.unwrap();

						assert!(value.as_str().ok().filter(|v| { *v == "HYDRO" }).is_some());

						let input = EvmDataWriter::new_with_selector(NativeAction::BalanceOf)
							.write(Address(evm_address))
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
						let info = rs.unwrap();

						// extract evm output value from raw bytes
						let value: Result<U256, _> = EvmDataReader::new(&info.value).read();
						assert_ok!(&value);

						assert!(value
							.ok()
							.filter(|v| {
								*v == U256::from(Currencies::free_balance(
									NATIVE_CURRENCY_ID,
									&mapped_account,
								))
							})
							.is_some());
					},
					_ => {
						panic!("shouldn't be of any other type");
					},
				}
			});
	}

	#[test]
	fn native_as_erc20() {
		let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
		let mapped_account =
			<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

		ExtBuilder::default()
			.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.sudo(mapped_account.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
			.build()
			.execute_with(|| {
				let contract = from_hex(&prepare_smart_contract("NativeERC20", "NativeToken"))
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
				enum IERC20Action {
					Name = "name()",
					Decimal = "decimal()",
					BalanceOf = "balanceOf(address)",
				}

				match &evt {
					// extract deployed address from pallet_evm::Event::Created
					pallet_evm::Event::Created(deployed_address) => {
						let input = EvmDataWriter::new_with_selector(IERC20Action::Name).build();
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
						let info = rs.unwrap();

						// extract evm output value from raw bytes
						let value: Result<precompile_utils::Bytes, _> =
							EvmDataReader::new(&info.value).read();
						assert_ok!(&value);

						let value = value.unwrap();

						assert!(value.as_str().ok().filter(|v| { *v == "HYDRO" }).is_some());

						let input = EvmDataWriter::new_with_selector(IERC20Action::BalanceOf)
							.write(Address(evm_address))
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
						let info = rs.unwrap();

						// extract evm output value from raw bytes
						let value: Result<U256, _> = EvmDataReader::new(&info.value).read();
						assert_ok!(&value);

						assert!(value
							.ok()
							.filter(|v| {
								*v == U256::from(Currencies::free_balance(
									NATIVE_CURRENCY_ID,
									&mapped_account,
								))
							})
							.is_some());
					},
					_ => {
						panic!("shouldn't be of any other type");
					},
				}
			});
	}
}
