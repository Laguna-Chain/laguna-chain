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
	use sp_core::{hexdisplay::AsBytesRef, Bytes};
	use std::str::FromStr;

	const LAGUNA_TOKEN: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
	const MAX_GAS: u64 = 200_000_000_000;

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
					deployer,
					contract,
				}) = &r.event
				{
					Some(contract)
				} else {
					None
				}
			})
			.expect("unable to found contract");

		deployed_address.clone()
	}

	#[test]
	fn test_ink_multilayer_erc20() {
		ExtBuilder::default()
			.balances(vec![(ALICE, LAGUNA_TOKEN, 10*LAGUNAS),(BOB, LAGUNA_TOKEN, 10*LAGUNAS),(EVA, LAGUNA_TOKEN, 10*LAGUNAS)])
			.build()
			.execute_with(|| {

				// 1. Deploy the library contract (native_fungible_token)
				let blob = std::fs::read("../integration-tests/contracts-data/ink/native_fungible_token/dist/native_fungible_token.wasm")
					.expect("Could not find wasm blob");

                let mut sel_constructor = Bytes::from_str("0xe0031b32")
                    .map(|v| v.to_vec())
                    .expect("unable to parse selector");

                sel_constructor.append(&mut 0_u32.encode());

                let erc20_contract_addr = deploy_contract(blob, sel_constructor);
				let native_token = CurrencyId::NativeToken(TokenId::Laguna);

				// 2. Test name()
				let sel_name = Bytes::from_str("0x3adaf70d")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_name.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let name = String::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(name, native_token.name());

				// 3. Test symbol()
				let sel_symbol = Bytes::from_str("0x9bd1933e")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_symbol.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let symbol = String::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(symbol, native_token.symbol());

				// 4. Test decimals()
				let sel_decimals = Bytes::from_str("0x81c09d87")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_decimals.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let decimals = u8::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(decimals, native_token.decimals());

				// 5. Test total_supply()
				let sel_total_supply = Bytes::from_str("0xdb6375a8")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_total_supply.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let total_supply = Balance::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(total_supply, Currencies::total_issuance(native_token));

				// 6. Test balance_of()
				let mut sel_balance_of = Bytes::from_str("0x0f755a56")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_balance_of.append(&mut ALICE.encode());

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_balance_of.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let alice_balance = Balance::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(alice_balance, Currencies::free_balance(ALICE, native_token));

				// 7. Test transfer()
				// @dev: EVA transfers BOB 10 LAGUNA
				let mut sel_transfer = Bytes::from_str("0x84a15da1")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_transfer.append(&mut BOB.encode());
				sel_transfer.append(&mut (5*LAGUNAS).encode());

				assert_ok!(Contracts::call(
					Origin::signed(EVA),
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_transfer.clone(),
				));

				assert_eq!(Currencies::free_balance(EVA, native_token), 5*LAGUNAS);
				assert_eq!(Currencies::free_balance(BOB, native_token), 15*LAGUNAS);

				// 8. Test allowance(BOB, ALICE)
				let mut sel_allowance = Bytes::from_str("0x6a00165e")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_allowance.append(&mut BOB.encode());
				sel_allowance.append(&mut ALICE.encode());

				let ExecReturnValue{flags, data} = <Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
					ALICE,
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_allowance.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let allowance = Balance::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(allowance, 0);

				// 9. Test approve()
				// @dev: BOB approves ALICE to spend upto 5 LAGUNA
				let mut sel_approve = Bytes::from_str("0x681266a0")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_approve.append(&mut ALICE.encode());
				sel_approve.append(&mut (5*LAGUNAS).encode());

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
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_allowance.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let allowance = Balance::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				assert_eq!(allowance, 5*LAGUNAS);

				// 10. Test transfer_from()
				// @dev: ALICE transfers 2 LAGUNA from BOB to EVA
				let mut sel_transfer_from = Bytes::from_str("0x0b396f18")
					.map(|v| v.to_vec())
					.expect("unable to parse hex string");

				sel_transfer_from.append(&mut BOB.encode());
				sel_transfer_from.append(&mut EVA.encode());
				sel_transfer_from.append(&mut (2*LAGUNAS).encode());

				let bob_balance_before = Currencies::free_balance(BOB, native_token);

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
					erc20_contract_addr.clone().into(),
					0,
					MAX_GAS,
					None,
					sel_allowance.clone(),
				)
				.result
				.expect("Execution without result");

				assert!(flags.is_empty());

				let allowance = Balance::decode(&mut data.as_bytes_ref()).expect("failed to decode result");
				let bob_balance_after = Currencies::free_balance(BOB, native_token);

				assert_eq!(bob_balance_before - bob_balance_after, 2*LAGUNAS);
				assert_eq!(allowance, 3*LAGUNAS);
				assert_eq!(Currencies::free_balance(EVA, native_token), 7*LAGUNAS);
			});
	}
}
