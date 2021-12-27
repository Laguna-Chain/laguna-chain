use fp_evm::PrecompileSet;
use frame_support::assert_ok;
use precompile_utils::EvmDataWriter;

use pallet_evm::Call as EvmCall;
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
        max_fee_per_gas: U256::max_value(),
        max_priority_fee_per_gas: None,
        access_list: vec![], // Use the next nonce
    }
}

#[test]
fn execute_native() {
    ExtBuilder::default()
        .balances(vec![(ALICE, 1000)])
        .build()
        .execute_with(|| {
            // runtime is able to successfully call the wrapped pallet
            assert_ok!(Call::Rando(pallet_rando::Call::dummy {}).dispatch(Origin::signed(ALICE)));
        });
}

#[test]
fn precompile_exist() {
    // address H160(1) should contain precompile
    assert!(Precompiles::<Runtime>::new().is_precompile(hash(1)));

    // address H160(2) shouldn't contain precompile
    assert!(!Precompiles::<Runtime>::new().is_precompile(hash(2)));
}

#[test]
fn execute_precompile_dummy() {
    ExtBuilder::default()
        .balances(vec![(ALICE, 1000)])
        .build()
        .execute_with(|| {
            // build the selector from Action
            let selector = EvmDataWriter::new_with_selector(Action::CallRando).build();

            // run the selector on the precompiled address of the wrapped module
            let rs =
                Precompiles::<Runtime>::new().execute(hash(1), &selector, None, &context(), false);

            // should have result from precoimpleset
            assert!(rs.is_some());
            let out = rs.unwrap();

            // execution should be done without error
            assert!(out.is_ok());
            let out: PrecompileOutput = out.unwrap();

            // precompile of PalletRando::dummy will stop after success execution
            assert_eq!(out.exit_status, ExitSucceed::Stopped);

            let selector = EvmDataWriter::new_with_selector(Action::GetCounts).build();

            // run the selector on the precompiled address of the wrapped module
            let rs =
                Precompiles::<Runtime>::new().execute(hash(1), &selector, None, &context(), false);

            // should have result from precoimpleset
            assert!(rs.is_some());
            let out = rs.unwrap();

            // execution should be done without error
            assert!(out.is_ok());
            let out: PrecompileOutput = out.unwrap();

            // precompile of PalletRando::get_counts have returning data
            assert_eq!(out.exit_status, ExitSucceed::Returned);

            let output = EvmDataWriter::new().write(1_u16).build();

            assert_eq!(out.output, output);
        });
}
