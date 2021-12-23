#![cfg_attr(not(feature = "std"), no_std)]

use std::marker::PhantomData;

use evm::{Context, ExitSucceed};
use fp_evm::{Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::AddressMapping;
use precompile_utils::{EvmDataReader, EvmResult, Gasometer, RuntimeHelper};

mod mock;

#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
enum Action {
    CallRando = "call_rando()",
}

pub struct RandoPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for RandoPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_rando::Config,
    Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    Runtime::Call: From<pallet_rando::Call<Runtime>>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> PrecompileResult {
        // TODO: extract input into rust struct for better composability

        // parse the evm selector from the action struct we defined, with the help of the generate_function_selector proc_macro
        let (input, selector) = EvmDataReader::new_with_selector(input)
            .map_err(|e| PrecompileFailure::Error { exit_status: e })?;

        // match evm function selector to pallet action
        match selector {
            Action::CallRando => Self::call_rando(context, target_gas)
                .map_err(|e| PrecompileFailure::Error { exit_status: e }),
        }
    }
}

// expose pallet_rando's internal Pallet::Call to the Precompile struct
impl<Runtime> RandoPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_rando::Config,
    Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    Runtime::Call: From<pallet_rando::Call<Runtime>>,
{
    fn call_rando(context: &Context, target_gas: Option<u64>) -> EvmResult<PrecompileOutput> {
        // create a gasometer to convert and calculate gas usage of this Pallet::Call
        let mut gasometer = Gasometer::new(target_gas);

        // extract account_id from the contract caller by calling the specified AddressMapping impl
        let origin = Runtime::AddressMapping::into_account_id(context.caller);

        // create a callable object waiting to be called
        let call = pallet_rando::Call::<Runtime>::dummy {};

        // execute the extrinsic and record gas usage
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;
        gasometer.record_cost(used_gas)?;

        // construct evm result from gas used and the extrinsic return
        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Stopped,
            cost: gasometer.used_gas(),
            output: Default::default(),
            logs: Default::default(),
        })
    }
}
