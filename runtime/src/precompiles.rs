// this is copied from frontier-workshop, we'll add our own later on

use frame_support::log;
use pallet_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use sp_core::H160;
use sp_std::marker::PhantomData;

use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_rando_precompile::RandoPrecompile;

pub struct HydroPrecompiles<R>(PhantomData<R>);

impl<Runtime> HydroPrecompiles<Runtime>
where
    Runtime: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self(Default::default())
    }
    pub fn used_addresses() -> sp_std::vec::Vec<H160> {
        sp_std::vec![1, 2, 3, 4, 5, 1024, 1025, 9001]
            .into_iter()
            .map(|x| hash(x))
            .collect()
    }
}

impl<Runtime> PrecompileSet for HydroPrecompiles<Runtime>
where
    Dispatch<Runtime>: Precompile,
    RandoPrecompile<Runtime>: Precompile,
    Runtime: pallet_evm::Config + pallet_rando::Config,
{
    fn execute(
        &self,
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> Option<PrecompileResult> {
        log::debug!("parsing precompile modules with the hashed addr");
        match address {
            // Ethereum precompiles :
            a if a == hash(1) => Some(ECRecover::execute(input, target_gas, context, is_static)),
            a if a == hash(2) => Some(Sha256::execute(input, target_gas, context, is_static)),
            a if a == hash(3) => Some(Ripemd160::execute(input, target_gas, context, is_static)),
            a if a == hash(4) => Some(Identity::execute(input, target_gas, context, is_static)),
            a if a == hash(5) => Some(Modexp::execute(input, target_gas, context, is_static)),
            // Non-Frontier specific nor Ethereum precompiles :
            a if a == hash(1024) => {
                Some(Sha3FIPS256::execute(input, target_gas, context, is_static))
            }
            a if a == hash(1025) => Some(ECRecoverPublicKey::execute(
                input, target_gas, context, is_static,
            )),
            a if a == hash(9001) => Some(RandoPrecompile::<Runtime>::execute(
                input, target_gas, context, is_static,
            )),
            a if a == hash(9002) => Some(Dispatch::<Runtime>::execute(
                input, target_gas, context, is_static,
            )),
            _ => {
                log::debug!("unmatched address: {:?}", address);
                None
            }
        }
    }

    fn is_precompile(&self, address: H160) -> bool {
        Self::used_addresses().contains(&address)
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
