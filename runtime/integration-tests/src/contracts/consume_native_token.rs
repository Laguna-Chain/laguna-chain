#[cfg(test)]
mod tests {
	use crate::{ExtBuilder, ALICE, BOB, EVA};
	use codec::{Decode, Encode};
	use frame_support::assert_ok;
	use laguna_runtime::{
		constants::LAGUNAS, Block, Contracts, Currencies, Event, Origin, Runtime, System,
	};
	use orml_traits::MultiCurrency;
	use pallet_contracts_primitives::ExecReturnValue;
	use pallet_contracts_rpc_runtime_api::runtime_decl_for_ContractsApi::ContractsApi;
	use primitives::{AccountId, Balance, BlockNumber, CurrencyId, Hash, TokenId, TokenMetadata};
	use sp_core::{hexdisplay::AsBytesRef, Bytes, U256};
	use sp_runtime::traits::AccountIdConversion;
	use std::str::FromStr;

	const LAGUNA_TOKEN: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
	const MAX_GAS: u64 = 200_000_000_000;

	fn deploy_system_contract(blob: Vec<u8>, sel_constructor: Vec<u8>) -> AccountId {
		assert_ok!(laguna_runtime::SystemContractDeployer::instantiate_with_code(
			Origin::root(),
			0,
			MAX_GAS,
			None,
			blob,
			sel_constructor,
			None,
		));

		let evts = System::events();

		let deployed_address = evts
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

		deployed_address.clone()
	}

	fn deploy_contract(blob: Vec<u8>, sel_constructor: Vec<u8>) -> AccountId {
		assert_ok!(Contracts::instantiate_with_code(
			Origin::signed(ALICE),
			0,
			MAX_GAS,
			None,
			blob,
			sel_constructor,
			vec![]
		));

		let evts = System::events();

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
			.expect("unable to find contract");

		deployed_address.clone()
	}

	#[test]
	fn test_ink_multilayer_erc20() {
		let deploying_key = <Runtime as pallet_system_contract_deployer::Config>::PalletId::get()
			.try_into_account()
			.expect("Invalid PalletId");
		ExtBuilder::default()
			.balances(vec![(ALICE, LAGUNA_TOKEN, 10*LAGUNAS),(BOB, LAGUNA_TOKEN, 10*LAGUNAS),(EVA, LAGUNA_TOKEN, 10*LAGUNAS), (deploying_key, LAGUNA_TOKEN, 10*LAGUNAS)])
			.build()
			.execute_with(|| {

				// 1. Deploy the library contract (native_fungible_token)
				let blob = std::fs::read("../integration-tests/contracts-data/ink/native_fungible_token/dist/native_fungible_token.wasm")
					.expect("Could not find wasm blob");

                let mut sel_constructor = Bytes::from_str("0x45fd0674")
                    .map(|v| v.to_vec())
                    .expect("unable to parse selector");

                sel_constructor.append(&mut 0_u32.encode());

				// Verify that non-root accounts cannot deploy an instance of native_fungible_token
				frame_support::assert_err_ignore_postinfo!(
					Contracts::instantiate_with_code(
						Origin::signed(ALICE),
						0,
						MAX_GAS,
						None,
						blob.clone(),
						sel_constructor.clone(),
						vec![]
					),
					pallet_contracts::Error::<Runtime>::ContractTrapped
				);

                let erc20_contract_addr = deploy_system_contract(blob, sel_constructor);

				// 2. Test name()
				let sel_name = Bytes::from_str("0x06fdde03")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_name,
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let name = String::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(name, LAGUNA_TOKEN.name());

				// 3. Test symbol()
				let sel_symbol = Bytes::from_str("0x95d89b41")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_symbol,
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let symbol = String::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(symbol, LAGUNA_TOKEN.symbol());

				// 4. Test decimals()
				let sel_decimals = Bytes::from_str("0x313ce567")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_decimals,
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let decimals = u8::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(decimals, LAGUNA_TOKEN.decimals());

				// 5. Test total_supply()
				let sel_total_supply = Bytes::from_str("0x18160ddd")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_total_supply.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let total_supply = U256::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(total_supply, Currencies::total_issuance(LAGUNA_TOKEN).into());

				// 6. Test balance_of()
				let mut sel_balance_of = Bytes::from_str("0x70a08231")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_balance_of.append(&mut ALICE.encode());

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_balance_of.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let alice_balance = U256::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(alice_balance, Currencies::free_balance(ALICE, LAGUNA_TOKEN).into());

				// 7. Test transfer()
				// @dev: EVA transfers BOB 10 LAGUNA
				let mut sel_transfer = Bytes::from_str("0xa9059cbb")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_transfer.append(&mut BOB.encode());
				sel_transfer.append(&mut U256::from(5*LAGUNAS).encode());

				assert_ok!(Contracts::call(
					Origin::signed(EVA),
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_transfer.clone(),
				));

				assert_eq!(Currencies::free_balance(EVA, LAGUNA_TOKEN), 5*LAGUNAS);
				assert_eq!(Currencies::free_balance(BOB, LAGUNA_TOKEN), 15*LAGUNAS);

				// 8. Test allowance(BOB, ALICE)
				let mut sel_allowance = Bytes::from_str("0xdd62ed3e")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_allowance.append(&mut BOB.encode());
				sel_allowance.append(&mut ALICE.encode());

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_allowance.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let allowance = U256::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(allowance, 0.into());

				// 9. Test approve()
				// @dev: BOB approves ALICE to spend upto 5 LAGUNA
				let mut sel_approve = Bytes::from_str("0x095ea7b3")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_approve.append(&mut ALICE.encode());
				sel_approve.append(&mut U256::from(5*LAGUNAS).encode());

				assert_ok!(Contracts::call(
					Origin::signed(BOB),
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_approve.clone(),
				));

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone(),
					0,
					MAX_GAS,
					None,
					sel_allowance.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let allowance = U256::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(allowance, (5*LAGUNAS).into());

				// 10. Test transfer_from()
				// @dev: ALICE transfers 2 LAGUNA from BOB to EVA
				let mut sel_transfer_from = Bytes::from_str("0x23b872dd")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_transfer_from.append(&mut BOB.encode());
				sel_transfer_from.append(&mut EVA.encode());
				sel_transfer_from.append(&mut U256::from(2*LAGUNAS).encode());

				let bob_balance_before = Currencies::free_balance(BOB, LAGUNA_TOKEN);

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_transfer_from.clone(),
				));

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.into(),
					0,
					MAX_GAS,
					None,
					sel_allowance.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let allowance = U256::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				let bob_balance_after = Currencies::free_balance(BOB, LAGUNA_TOKEN);

				assert_eq!(bob_balance_before - bob_balance_after, 2*LAGUNAS);
				assert_eq!(allowance, (3*LAGUNAS).into());
				assert_eq!(Currencies::free_balance(EVA, LAGUNA_TOKEN), 7*LAGUNAS);
			});
	}

	#[test]
	fn test_solang_multilayer_amm() {
		let deploying_key = <Runtime as pallet_system_contract_deployer::Config>::PalletId::get()
			.try_into_account()
			.expect("Invalid PalletId");
		ExtBuilder::default()
			.balances(vec![(ALICE, LAGUNA_TOKEN, 1000*LAGUNAS), (deploying_key, LAGUNA_TOKEN, 10*LAGUNAS)])
			.build()
			.execute_with(|| {
				// @NOTE: Just a simple test method to verify multilayer interaction and ERC20 works!
				// Does not do extensive test coverage for the contract - AMM

				// 1A. Deploy the library contract (native_fungible_token)
				let blob_native_erc20 = std::fs::read("../integration-tests/contracts-data/ink/native_fungible_token/dist/native_fungible_token.wasm")
					.expect("Could not find wasm blob");

                let mut sel_constructor_native_erc20 = Bytes::from_str("0x45fd0674")
                    .map(|v| v.to_vec())
                    .expect("unable to parse selector");

                sel_constructor_native_erc20.append(&mut 0_u32.encode());

                let native_erc20_addr = deploy_system_contract(blob_native_erc20, sel_constructor_native_erc20);

				// 1B. Deploy a standard ERC20 contract (ERC20)
				let blob_std_erc20 = std::fs::read("../integration-tests/contracts-data/solidity/erc20/dist/ERC20.wasm")
					.expect("Could not find wasm blob");

                let mut sel_constructor_std_erc20 = Bytes::from_str("0x835a15cb")
                    .map(|v| v.to_vec())
                    .expect("unable to parse selector");

                sel_constructor_std_erc20.append(&mut "Ethereum".encode());
				sel_constructor_std_erc20.append(&mut "ETH".encode());
				sel_constructor_std_erc20.append(&mut U256::exp10(32).encode());

                let std_erc20_addr = deploy_contract(blob_std_erc20, sel_constructor_std_erc20);

				// 1C. Deploy test dAPP (AMM)
				let blob_amm = std::fs::read("../integration-tests/contracts-data/solidity/amm/dist/AMM.wasm")
					.expect("Could not find wasm blob");

                let mut sel_constructor_amm = Bytes::from_str("0x1c26cc85")
                    .map(|v| v.to_vec())
                    .expect("unable to parse selector");

                sel_constructor_amm.append(&mut native_erc20_addr.encode());
				sel_constructor_amm.append(&mut std_erc20_addr.encode());

                let amm_addr = deploy_contract(blob_amm, sel_constructor_amm);

				// 2. Approve AMM contract to spend tokens on ALICE's behalf
				let mut sel_approve = Bytes::from_str("0x095ea7b3")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_approve.append(&mut amm_addr.encode());
				sel_approve.append(&mut U256::MAX.encode());

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					native_erc20_addr.into(),
					0,
					MAX_GAS,
					None,
					sel_approve.clone(),
				));

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					std_erc20_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_approve.clone(),
				));

				let get_balance = |account| {
					let mut sel_balance_of = Bytes::from_str("0x70a08231")
						.map(|v| v.to_vec())
						.expect("unable to parse hex string");

					sel_balance_of.append(&mut ALICE.encode());

					let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						account,
						std_erc20_addr.clone(),
						0,
						MAX_GAS,
						None,
						sel_balance_of.clone(),
					)
					.result
					.expect("Execution without result");

					assert!(flags.is_empty());

					let std_erc20_bal = U256::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
					let native_erc20_bal: U256 = Currencies::free_balance(ALICE, LAGUNA_TOKEN).into();
					(native_erc20_bal, std_erc20_bal)
				};

				// 3. "Add liquidity" works
				let mut sel_provide = Bytes::from_str("0xe8c3c54f")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				// 1 Native = 10_000 Standard
				sel_provide.append(&mut U256::exp10(6).encode());
				sel_provide.append(&mut U256::exp10(10).encode());

				let (native_bal_before, std_bal_before) = get_balance(ALICE);
				println!("Native balance before PROVIDE => {:?}", native_bal_before);
				println!("Standard balance before PROVIDE => {:?}\n", std_bal_before);

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					amm_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_provide.clone(),
				));

				let (native_bal_after, std_bal_after) = get_balance(ALICE);
				println!("Native balance after PROVIDE => {:?}", native_bal_after);
				println!("Standard balance after PROVIDE => {:?}\n", std_bal_after);

				assert_eq!(native_bal_before - native_bal_after - 600_960_000_000_u128, U256::exp10(6)); // Adjusting fees
				assert_eq!(std_bal_before - std_bal_after, U256::exp10(10));

				// 4. "Remove liquidity" works
				let mut sel_withdraw = Bytes::from_str("0x2e1a7d4d")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_withdraw.append(&mut U256::exp10(6).encode());

				let (native_bal_before, std_bal_before) = get_balance(ALICE);
				println!("Native balance before WITHDRAW => {:?}", native_bal_before);
				println!("Standard balance before WITHDRAW => {:?}\n", std_bal_before);

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					amm_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_withdraw.clone(),
				));

				let (native_bal_after, std_bal_after) = get_balance(ALICE);
				println!("Native balance after WITHDRAW => {:?}", native_bal_after);
				println!("Standard balance after WITHDRAW => {:?}\n", std_bal_after);

				assert_eq!(native_bal_after - native_bal_before, U256::exp10(4));
				assert_eq!(std_bal_after - std_bal_before, U256::exp10(8));

				// 5A. Swap (STD to NATIVE) works
				let mut sel_swap = Bytes::from_str("0x980d69d3")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_swap.append(&mut U256::exp10(4).encode());

				let (native_bal_before, std_bal_before) = get_balance(ALICE);
				println!("Native balance before SWAP => {:?}", native_bal_before);
				println!("Standard balance before SWAP => {:?}\n", std_bal_before);

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					amm_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_swap.clone(),
				));

				let (native_bal_after, std_bal_after) = get_balance(ALICE);
				println!("Native balance after SWAP => {:?}", native_bal_after);
				println!("Standard balance after SWAP => {:?}\n", std_bal_after);

				assert_eq!(native_bal_after - native_bal_before, U256::exp10(0));
				assert_eq!(std_bal_before - std_bal_after, U256::exp10(4));

				// 5B. Swap (NATIVE to STD) works
				let mut sel_swap = Bytes::from_str("0xf4cb34d4")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_swap.append(&mut U256::exp10(0).encode());

				let (native_bal_before, std_bal_before) = get_balance(ALICE);
				println!("Native balance before SWAP => {:?}", native_bal_before);
				println!("Standard balance before SWAP => {:?}\n", std_bal_before);

				assert_ok!(Contracts::call(
					Origin::signed(ALICE),
					amm_addr.into(),
					0,
					MAX_GAS,
					None,
					sel_swap.clone(),
				));

				let (native_bal_after, std_bal_after) = get_balance(ALICE);
				println!("Native balance after SWAP => {:?}", native_bal_after);
				println!("Standard balance after SWAP => {:?}\n", std_bal_after);

				assert_eq!(native_bal_before - native_bal_after, U256::exp10(0));
				assert_eq!(std_bal_after - std_bal_before, U256::exp10(4));
			});
	}
}
