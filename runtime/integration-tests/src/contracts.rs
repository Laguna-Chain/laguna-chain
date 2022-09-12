mod chain_extension;
mod consume_native_token;
mod cross_contract;

#[cfg(test)]
mod tests {
	use crate::{ExtBuilder, ALICE};
	use codec::{Decode, Encode};
	use frame_support::assert_ok;
	use laguna_runtime::{
		constants::LAGUNAS, Block, Contracts, Event, Origin, Runtime, System,
		SystemContractDeployer,
	};
	use pallet_contracts_primitives::ExecReturnValue;
	use pallet_contracts_rpc_runtime_api::runtime_decl_for_ContractsApi::ContractsApi;
	use primitives::{AccountId, Balance, BlockNumber, CurrencyId, Hash, TokenId};
	use sp_core::{crypto::AccountId32, hexdisplay::AsBytesRef, Bytes};
	use sp_runtime::traits::AccountIdConversion;
	use std::str::FromStr;

	const LAGUNA_TOKEN: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
	const MAX_GAS: u64 = 200_000_000_000;

	#[test]
	fn test_ink_basic() {
		ExtBuilder::default()
			.balances(vec![(ALICE, LAGUNA_TOKEN, 10 * LAGUNAS)])
			.build()
			.execute_with(|| {
				let mut acc_counter = 0_u32; // needed to avoid account_id duplication, should generat random salt in production

				let blob =
					std::fs::read("../integration-tests/contracts-data/ink/basic/dist/basic.wasm")
						.expect("cound not find wasm blob");

				// constructor with not argument
				let sel_constructor = Bytes::from_str("0xed4b9d1b")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				assert_ok!(Contracts::instantiate_with_code(
					Origin::signed(ALICE),
					0,
					MAX_GAS,
					None,
					blob.clone(),
					sel_constructor,
					acc_counter.encode(),
				));

				let evts = System::events();

				// deployed contract can be found in the last created instantiated event
				let deployed_address = evts
					.iter()
					.rev()
					.find_map(|r| {
						if let Event::Contracts(pallet_contracts::Event::Instantiated {
							deployer: _,
							contract,
						}) = &r.event
						{
							Some(contract)
						} else {
							None
						}
					})
					.expect("unable to find the last deployed contract");

				acc_counter += 1;

				// prepare the getter
				let sel_getter = Bytes::from_str("0x2f865bd9")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				// use ContractsApi on the Runtime to query result of a read method
				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone(),
						0,
						MAX_GAS,
						None,
						sel_getter.clone(),
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				// empty flags determines succesful execution
				assert!(flags.is_empty());

				assert!(bool::decode(&mut data.as_bytes_ref()).ok().filter(|rs| !(*rs)).is_some());

				let sel_flip = Bytes::from_str("0x633aa551")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				// submit call on the getter
				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					deployed_address.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_flip.clone(),
				));

				// read the getter again after state mutating call
				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone(),
						0,
						MAX_GAS,
						None,
						sel_getter.clone(),
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				assert!(flags.is_empty());

				// assert state changes
				assert!(bool::decode(&mut data.as_bytes_ref()).ok().filter(|rs| *rs).is_some());

				let mut sel_constructor_with_arg = Bytes::from_str("0x9bae9d5e")
					.map(|v| v.to_vec())
					.expect("unable to parse hex str");

				sel_constructor_with_arg.append(&mut true.encode());

				assert_ok!(Contracts::instantiate_with_code(
					Origin::signed(ALICE),
					0,
					MAX_GAS,
					None,
					blob,
					sel_constructor_with_arg,
					acc_counter.encode()
				));

				let evts = System::events();

				// deployed contract can be found in the last created instantiated event
				let deployed_address = evts
					.iter()
					.rev()
					.find_map(|r| {
						if let Event::Contracts(pallet_contracts::Event::Instantiated {
							deployer: _,
							contract,
						}) = &r.event
						{
							Some(contract)
						} else {
							None
						}
					})
					.expect("unable to find the last deployed contract");

				// read the getter again before state mutating call
				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone(),
						0,
						MAX_GAS,
						None,
						sel_getter.clone(),
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				assert!(flags.is_empty());

				// assert state changes
				assert!(bool::decode(&mut data.as_bytes_ref()).ok().filter(|rs| *rs).is_some());

				// submit call on the getter
				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					deployed_address.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_flip,
				));

				// read the getter again before state mutating call
				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone(),
						0,
						MAX_GAS,
						None,
						sel_getter,
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				assert!(flags.is_empty());

				// assert state changes
				assert!(bool::decode(&mut data.as_bytes_ref()).ok().filter(|rs| !(*rs)).is_some());
			});
	}

	#[test]
	fn test_sol_basic() {
		ExtBuilder::default()
			.balances(vec![(ALICE, LAGUNA_TOKEN, 10 * LAGUNAS)])
			.build()
			.execute_with(|| {
				let acc_counter = 0_u32; // needed to avoid account_id duplication, should generat random salt in production

				let blob = std::fs::read(
					"../integration-tests/contracts-data/solidity/basic/dist/Basic.wasm",
				)
				.expect("cound not find wasm blob");

				// constructor with not argument
				let mut sel_constructor = Bytes::from_str("0xf81e7e1a")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_constructor.append(&mut true.encode());

				assert_ok!(Contracts::instantiate_with_code(
					Origin::signed(ALICE),
					0,
					MAX_GAS,
					None,
					blob,
					sel_constructor,
					acc_counter.encode(),
				));

				let evts = System::events();

				// deployed contract can be found in the last created instantiated event
				let deployed_address = evts
					.iter()
					.rev()
					.find_map(|r| {
						if let Event::Contracts(pallet_contracts::Event::Instantiated {
							deployer: _,
							contract,
						}) = &r.event
						{
							Some(contract)
						} else {
							None
						}
					})
					.expect("unable to find the last deployed contract");

				// prepare the getter
				let sel_getter = Bytes::from_str("0x20965255")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				// use ContractsApi on the Runtime to query result of a read method
				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone(),
						0,
						MAX_GAS,
						None,
						sel_getter.clone(),
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				// empty flags determines succesful execution
				assert!(flags.is_empty());

				assert!(bool::decode(&mut data.as_bytes_ref()).ok().filter(|rs| *rs).is_some());

				let sel_flip = Bytes::from_str("0xcde4efa9")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				// submit call on the getter
				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					deployed_address.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_flip,
				));

				// read the getter again after state mutating call
				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone(),
						0,
						MAX_GAS,
						None,
						sel_getter,
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				assert!(flags.is_empty());

				// assert state changes
				assert!(bool::decode(&mut data.as_bytes_ref()).ok().filter(|rs| !(*rs)).is_some());
			});
	}

	#[test]
	fn test_fixed_address() {
		let deploying_key = <Runtime as pallet_system_contract_deployer::Config>::PalletId::get()
			.try_into_account()
			.expect("Invalid PalletId");
		ExtBuilder::default()
			.balances(vec![(ALICE, LAGUNA_TOKEN, LAGUNAS), (deploying_key, LAGUNA_TOKEN, LAGUNAS)])
			.sudo(ALICE)
			.build()
			.execute_with(|| {
				let blob =
					std::fs::read("../integration-tests/contracts-data/ink/basic/dist/basic.wasm")
						.expect("cound not find wasm blob");

				let sel_constructor = Bytes::from_str("0xed4b9d1b")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				assert_ok!(laguna_runtime::SystemContractDeployer::instantiate_with_code(
					Origin::root(),
					0,
					MAX_GAS,
					None,
					blob,
					sel_constructor,
					Some([0x11; 32]),
				));

				let evts = System::events();

				let deployed_addr = evts
					.iter()
					.rev()
					.find_map(|r| {
						if let Event::SystemContractDeployer(
							pallet_system_contract_deployer::Event::Created(contract),
						) = &r.event
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
}
