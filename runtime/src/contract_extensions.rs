use frame_support::{log::error, sp_runtime::DispatchError};
use pallet_contracts::chain_extension::{
	ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};

use super::Runtime;

pub struct DemoExtension;

impl ChainExtension<Runtime> for DemoExtension {
	fn call<E>(
		&mut self,
		env: Environment<E, InitState>,
	) -> pallet_contracts::chain_extension::Result<RetVal>
	where
		E: Ext<T = Runtime>,
		<E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
	{
		let func_id = env.func_id();
		match func_id {
			1000 => {
				let mut env = env.buf_in_buf_out();
				let arg: [u8; 32] = env.read_as()?;
				env.write(&arg, false, None)
					.map_err(|_| DispatchError::Other("ChainExtension failed to call demo"))?;
			},
			_ => {
				error!("Called an unregistered `func_id`: {:}", func_id);
				return Err(DispatchError::Other("Unimplemented func_id"))
			},
		}

		Ok(RetVal::Converging(0))
	}

	fn enabled() -> bool {
		true
	}
}
