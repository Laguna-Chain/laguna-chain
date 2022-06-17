use super::Runtime;
use crate::Currencies;
use codec::Encode;
use frame_support::log::error;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use pallet_contracts::chain_extension::{
	ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use sp_runtime::DispatchError;

fn allowance(asset: CurrencyId, owner: AccountId, spender: AccountId) -> Balance {
	unimplemented!()
}

fn transfer(asset: CurrencyId, from: AccountId, to: AccountId, value: Balance) -> u32 {
	let origin = RawOrigin::Signed(from);
	match Currencies::transfer(origin.into(), to, asset, value) {
		Ok(_) => 0,
		Err(_) => 2,
	}
}

fn approve(asset: CurrencyId, owner: AccountId, spender: AccountId, value: Balance) -> u32 {
	unimplemented!()
}

fn transfer_from(
	asset: CurrencyId,
	caller: AccountId,
	from: AccountId,
	to: AccountId,
	value: Balance,
) -> u32 {
	// 1. Call allowance() (from => caller) to check authorisation
	// 2. Call Transfer() (from => to) and verify (Reentrancy possible?)
	// 3. Call approve() (from => caller) to update allowance
	unimplemented!()
}

pub struct DemoExtension;

impl ChainExtension<Runtime> for DemoExtension {
	fn call<E>(
		func_id: u32,
		env: Environment<E, InitState>,
	) -> pallet_contracts::chain_extension::Result<RetVal>
	where
		E: Ext<T = Runtime>,
		<E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
	{
		let mut env = env.buf_in_buf_out();
		match func_id {
			1000 => {
				let arg: [u8; 32] = env.read_as()?;
				env.write(&arg, false, None)
					.map_err(|_| DispatchError::Other("ChainExtension failed to call demo"))?;
				Ok(RetVal::Converging(0))
			},
			_ if 2000 <= func_id && func_id < 3000 => {
				// Native token access as ERC20 token
				let token_id: u32 = env.read_as()?;
				let currency = CurrencyId::NativeToken(match token_id {
					0 => TokenId::Laguna,
					1 => TokenId::FeeToken,
					_ => return Ok(RetVal::Converging(1)),
				});

				match func_id {
					2000 => Ok(RetVal::Converging(0)),
					2001 => {
						// Get total supply
						let supply = Currencies::total_issuance(currency);
						env.write(&supply.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call total_supply")
						})?;
						Ok(RetVal::Converging(0))
					},
					2002 => {
						// Get balance
						let account: AccountId = env.read_as()?;

						let balance = Currencies::free_balance(account, currency);
						env.write(&balance.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call balance_of")
						})?;
						Ok(RetVal::Converging(0))
					},
					2003 => {
						// Get Allowance
						let owner: AccountId = env.read_as()?;
						let spender: AccountId = env.read_as()?;

						let allowance = allowance(currency, owner, spender);
						env.write(&allowance.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call allowance")
						})?;
						Ok(RetVal::Converging(0))
					},
					2004 => {
						// Transfer tokens
						let from: AccountId = env.ext().caller().clone();
						let to: AccountId = env.read_as()?;
						let value: Balance = env.read_as()?;

						let err_code = transfer(currency, from, to, value);
						Ok(RetVal::Converging(err_code))
					},
					2005 => {
						// Set allowance
						let owner: AccountId = env.ext().caller().clone();
						let spender: AccountId = env.read_as()?;
						let value: Balance = env.read_as()?;

						let err_code = approve(currency, owner, spender, value);
						Ok(RetVal::Converging(err_code))
					},
					2006 => {
						// transfer_from
						let caller: AccountId = env.ext().caller().clone();
						let from: AccountId = env.read_as()?;
						let to: AccountId = env.read_as()?;
						let value: Balance = env.read_as()?;

						let err_code = transfer_from(currency, caller, from, to, value);
						Ok(RetVal::Converging(err_code))
					},
					_ => {
						error!("Called an unregistered `func_id`: {:}", func_id);
						Err(DispatchError::Other("Unimplemented func_id"))
					},
				}
			},
			_ => {
				error!("Called an unregistered `func_id`: {:}", func_id);
				Err(DispatchError::Other("Unimplemented func_id"))
			},
		}
	}

	fn enabled() -> bool {
		true
	}
}
