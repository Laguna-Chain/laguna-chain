//! native_token_precompile
//!
//! test if native_token are able to be queried and moved via the precompile address

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use hydro_runtime::{constants::HYDROS, precompiles::HydroPrecompiles, Currencies, Runtime};
	use precompile_utils::{Address, Bytes, EvmDataReader, EvmDataWriter};
	use sp_core::{H160, U256};

	use crate::{ExtBuilder, ALICE, NATIVE_CURRENCY_ID};
	use orml_traits::MultiCurrency;
	use pallet_evm::{AddressMapping, Context, PrecompileSet};

	#[test]
	fn read_native_name() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.build()
			.execute_with(|| {
				let selector =
					EvmDataWriter::new_with_selector(native_asset_precompile::Action::GetName)
						.build();

				let output = HydroPrecompiles::<Runtime>::new().execute(
					H160::from_low_u64_be(9002),
					&selector,
					None,
					&Context {
						apparent_value: From::from(0_u64),
						address: Default::default(),
						caller: Default::default(),
					},
					false,
				);

				assert!(output.is_some());
				let result = output.unwrap();

				assert!(result.is_ok());
				let rv = result.unwrap();

				let expected_return = EvmDataWriter::new().write(Bytes::from("HYDRO")).build();
				assert_eq!(rv.output, expected_return);

				let unexpected_return =
					EvmDataWriter::new().write(Bytes::from("NOT_HYDRO")).build();
				assert!(rv.output != unexpected_return);
			});
	}

	#[test]
	fn read_native_amount() {
		// TODO: test bidirectional fundign when we have account auto claiming feature

		// genrate AccountId from evm addres
		let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
		let mapped_account =
			<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

		ExtBuilder::default()
			.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.build()
			.execute_with(|| {
				let init_amount = Currencies::free_balance(NATIVE_CURRENCY_ID, &mapped_account);
				assert_eq!(init_amount, 10 * HYDROS);

				let selector =
					EvmDataWriter::new_with_selector(native_asset_precompile::Action::BalanceOf)
						.write(Address(evm_address))
						.build();

				let output = HydroPrecompiles::<Runtime>::new().execute(
					H160::from_low_u64_be(9002),
					&selector,
					None,
					&Context {
						apparent_value: From::from(0_u64),
						address: Default::default(),
						caller: Default::default(),
					},
					false,
				);

				assert!(output.is_some());
				let result = output.unwrap();

				assert!(result.is_ok());
				let rv = result.unwrap();

				let output_result: Result<U256, _> = EvmDataReader::new(&rv.output).read();
				assert_ok!(&output_result);

				let output_amount = output_result.unwrap();
				assert_eq!(U256::from(init_amount), output_amount);

				// test if slashing is observable from precompile address
				assert!(Currencies::slash(NATIVE_CURRENCY_ID, &mapped_account, 1 * HYDROS) == 0);
				assert_eq!(
					Currencies::free_balance(NATIVE_CURRENCY_ID, &mapped_account),
					init_amount - HYDROS
				);

				// read through selector again
				let output = HydroPrecompiles::<Runtime>::new().execute(
					H160::from_low_u64_be(9002),
					&selector,
					None,
					&Context {
						apparent_value: From::from(0_u64),
						address: Default::default(),
						caller: Default::default(),
					},
					false,
				);

				assert!(output.is_some());
				let result = output.unwrap();

				assert!(result.is_ok());
				let rv = result.unwrap();

				let output_result: Result<U256, _> = EvmDataReader::new(&rv.output).read();
				assert_ok!(&output_result);

				// check whether free_amount read from substrate and evm is the same
				let output_amount = output_result.unwrap();
				assert_eq!(
					U256::from(Currencies::free_balance(NATIVE_CURRENCY_ID, &mapped_account)),
					output_amount
				);
			});
	}
}
