/// ## chain-extension tests
///
/// Considering tests for the following scopes:
/// 1. expose single runtime feature to a contract
/// 2. expose multiple runtime features to a contract

#[cfg(test)]
mod tests {

	use crate::{ExtBuilder, ALICE};
	use frame_support::assert_ok;
	use hydro_runtime::{constants::HYDROS, Block, Contracts, Event, Origin, Runtime, System};
	use pallet_contracts_primitives::ExecReturnValue;
	use pallet_contracts_rpc_runtime_api::runtime_decl_for_ContractsApi::ContractsApi;
	use primitives::{AccountId, Balance, BlockNumber, CurrencyId, Hash, TokenId};
	use sp_core::Bytes;
	use std::str::FromStr;

	const HYDRO_TOKEN: CurrencyId = CurrencyId::NativeToken(TokenId::Hydro);
	const MAX_GAS: u64 = 200_000_000_000;

	#[test]
	fn test_dummy_extension() {
		ExtBuilder::default()
			.balances(vec![(ALICE, HYDRO_TOKEN, 10 * HYDROS)])
			.build()
			.execute_with(|| {

				let blob = std::fs::read("../integration-tests/contracts-data/ink/dummy_extension_consumer/dist/dummy_extension_consumer.wasm").expect("unable to find wasm blob");

				let sel_constructor = Bytes::from_str("0xed4b9d1b").map(|v| v.to_vec()).expect("unable to parse selector");

				assert_ok!(
					Contracts::instantiate_with_code(
						Origin::signed(ALICE),
						0,
						MAX_GAS,
						None,
						blob,
						sel_constructor,
						vec![]
					)
				);

				let evts = System::events();

				let deployed_address = evts.iter().rev().find_map(|r| {

					if let Event::Contracts(pallet_contracts::Event::Instantiated { deployer, contract }) = &r.event {
						Some(contract)
					} else {
						None
					}
				}).expect("unable to found contract");


				let mut sel_ext = Bytes::from_str("0x35a21ae9").map(|v| v.to_vec()).expect("unable to parse selector");


				let input = [0_u8; 32];
				sel_ext.append(&mut input.to_vec());

				let rs =
					<Runtime as ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>>::call(
						ALICE,
						deployed_address.clone().into(),
						0,
						MAX_GAS,
						None,
						sel_ext,
					)
					.result
					.expect("execution without result");

				let ExecReturnValue { flags, data } = rs;

				// should execute succesfully
				assert!(flags.is_empty());

				// the first byte could be used to determine return status
				assert_eq!(data.to_vec(), [0_u8; 33]);
			});
	}
}
