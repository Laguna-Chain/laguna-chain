#![cfg_attr(not(feature = "std"), no_std)]

use std::marker::PhantomData;

use codec::Decode;
use evm::{Context, ExitError, ExitSucceed};
use fp_evm::PrecompileResult;
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};

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
    ) -> pallet_evm::PrecompileResult {
        const SELECTOR_SIZE_BYTES: usize = 4;

        todo!()
    }
}
