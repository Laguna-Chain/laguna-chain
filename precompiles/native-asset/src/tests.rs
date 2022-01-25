use fp_evm::PrecompileSet;
use frame_support::assert_ok;
use precompile_utils::EvmDataWriter;

use pallet_evm::{Call as EvmCall, Runner};
use sp_core::U256;

use crate::mock::*;

use super::*;

fn evm_call(input: Vec<u8>) -> EvmCall<Runtime> {
	EvmCall::call {
		source: alice(),
		target: hash(1),
		input,
		value: U256::zero(), // No value sent in EVM
		gas_limit: u64::max_value(),
		nonce: None,
		max_fee_per_gas: 0.into(),
		max_priority_fee_per_gas: Some(U256::zero()),
		access_list: vec![], // Use the next nonce
	}
}

#[test]
fn precompile_exist() {
	// address H160(1) should contain precompile
	assert!(Precompiles::<Runtime>::new().is_precompile(hash(1)));

	// address H160(2) shouldn't contain precompile
	assert!(!Precompiles::<Runtime>::new().is_precompile(hash(2)));
}

#[test]
fn precompile_get_name() {
	ExtBuilder::default().balances(vec![(ALICE, 1000)]).build().execute_with(|| {
		let selector = EvmDataWriter::new_with_selector(Action::GetName).build();

		// run the selector on the precompiled address of the wrapped module
		let rs = Precompiles::<Runtime>::new().execute(hash(1), &selector, None, &context(), false);

		// should have result from precoimpleset
		assert!(rs.is_some());
		let out = rs.unwrap();

		// execution should be done without error
		assert!(out.is_ok());
		let out: PrecompileOutput = out.unwrap();

		let expected = EvmDataWriter::new().write(Bytes::from("HYDRO")).build();
		assert_eq!(out.output, expected);

		let unexpected = EvmDataWriter::new().write(Bytes::from("NOT_HYDRO")).build();
		assert!(out.output != unexpected);
	});
}

#[test]
fn precompile_get_balance() {
	let init_amount = 1000;
	ExtBuilder::default()
		.balances_evm(vec![(alice(), init_amount)]) // prefund account_id mapped to H160(alice)
		.build()
		.execute_with(|| {
			// query balance of H160(alice)
			let selector = EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write(Address(alice()))
				.build();

			// run the selector on the precompiled address of the wrapped module
			let rs =
				Precompiles::<Runtime>::new().execute(hash(1), &selector, None, &context(), false);

			// should have result from precoimpleset
			assert!(rs.is_some());
			let out = rs.unwrap();

			// execution should be done without error
			assert!(out.is_ok());
			let out: PrecompileOutput = out.unwrap();

			let res: Result<U256, _> = EvmDataReader::new(&out.output).read();
			assert_ok!(&res);

			let amount = res.unwrap();

			assert!(amount == U256::from(init_amount));
		});
}

#[test]
fn precompile_get_balance_evm() {
	let init_amount = 1000;
	ExtBuilder::default()
		.balances_evm(vec![(alice(), init_amount)]) // prefund account_id mapped to H160(alice)
		.build()
		.execute_with(|| {
			// query balance of H160(alice)
			let selector = EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write(Address(alice()))
				.build();

			// raw evm execution using T::Runner, for inspecting evm function output
			let result = <Runtime as pallet_evm::Config>::Runner::call(
				alice(),
				hash(1),
				selector,
				0_u64.into(),
				u64::MAX,
				None,
				None,
				None,
				vec![],
				<Runtime as pallet_evm::Config>::config(),
			);

			assert_ok!(&result);
			let info = result.unwrap();

			// read output bytes as evm::U256
			let res: Result<U256, _> = EvmDataReader::new(&info.value).read();
			assert_ok!(&res);

			let amount = res.unwrap();
			assert!(amount == U256::from(init_amount));
		});
}
