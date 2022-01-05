#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::{
    Context, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    log,
    pallet_prelude::PhantomData,
};
use pallet_evm::AddressMapping;
use precompile_utils::{
    Address, Bytes, EvmDataReader, EvmDataWriter, EvmResult, Gasometer, RuntimeHelper,
};
use sp_core::{H160, U256};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// interaface for query native currency, expected to be consumed by erc20 contracts
#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    GetName = "name()",
    GetSymbol = "symbol()",
    GetDecimals = "decimals()",
    TotalSupply = "totalSupply()",
    BalanceOf = "balanceOf(address)",
    Transfer = "transfer(address,address,uint256)",
}

pub struct NativeCurrencyPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for NativeCurrencyPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_balances::Config,
    Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    sp_core::U256: From<<Runtime as pallet_balances::Config>::Balance>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> fp_evm::PrecompileResult {
        let (input, selector) = EvmDataReader::new_with_selector::<Action>(input).map_err(|e| {
            log::debug!("parsing failed");
            PrecompileFailure::Error { exit_status: e }
        })?;

        let rs = match selector {
            Action::GetName => Self::get_name(),
            Action::GetSymbol => todo!(),
            Action::GetDecimals => todo!(),
            Action::TotalSupply => todo!(),
            Action::BalanceOf => Self::balance_of(&context, input, target_gas),
            Action::Transfer => todo!(),
        }
        .map_err(|e| PrecompileFailure::Error { exit_status: e });

        log::debug!("{:?}", rs);

        rs
    }
}

impl<Runtime> NativeCurrencyPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_balances::Config,
    Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    sp_core::U256: From<<Runtime as pallet_balances::Config>::Balance>,
{
    fn get_name() -> EvmResult<PrecompileOutput> {
        let output = EvmDataWriter::new()
            .write::<Bytes>("HYDRO".into())
            .build()
            .into();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: Default::default(),
            output,
            logs: Default::default(),
        })
    }

    fn balance_of(
        context: &Context,
        mut input: EvmDataReader,
        target_gas: Option<u64>,
    ) -> EvmResult<PrecompileOutput> {
        // create a gasometer to convert and calculate gas usage of this Pallet::Call
        let mut gasometer = Gasometer::new(target_gas);

        // check input length
        input.expect_arguments(1)?;

        // read H160 from input
        let target_address: H160 = input.read::<Address>()?.into();

        let amount: U256 = {
            // get account_id from H160
            let target_account_id: Runtime::AccountId =
                Runtime::AddressMapping::into_account_id(target_address);

            // get native balance from pallet_balances
            pallet_balances::Pallet::<Runtime>::usable_balance(&target_account_id).into()
        };

        gasometer.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        let output = EvmDataWriter::new().write(amount).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }
}
