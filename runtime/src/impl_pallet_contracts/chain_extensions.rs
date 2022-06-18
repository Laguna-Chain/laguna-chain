use super::Runtime;
use crate::Currencies;
use codec::Encode;
use frame_support::log::error;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use pallet_contracts::chain_extension::{
	ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use primitives::{AccountId, Balance, CurrencyId, TokenId, TokenMetadata};
use sp_runtime::DispatchError;

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
			0010 => {
				// @todo: Whitelist contract after verification
				unimplemented!()
			},
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
					_ => return Ok(RetVal::Converging(1)), // Err::InvalidTokenId
				});

				match func_id {
					2000 => Ok(RetVal::Converging(0)),
					2001 => {
						// Get token name
						let name = currency.name();
						env.write(&name.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call allowance")
						})?;
						Ok(RetVal::Converging(0))
					},
					2002 => {
						// Get token symbol
						let symbol = currency.symbol();
						env.write(&symbol.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call allowance")
						})?;
						Ok(RetVal::Converging(0))
					},
					2003 => {
						// Get token decimals
						let decimals = currency.decimals();
						env.write(&decimals.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call allowance")
						})?;
						Ok(RetVal::Converging(0))
					},
					2004 => {
						// Get total supply
						let supply = Currencies::total_issuance(currency);
						env.write(&supply.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call total_supply")
						})?;
						Ok(RetVal::Converging(0))
					},
					2005 => {
						// Get balance
						let account: AccountId = env.read_as()?;

						let balance = Currencies::free_balance(account, currency);
						env.write(&balance.encode(), false, None).map_err(|_| {
							DispatchError::Other("ChainExtension failed to call balance_of")
						})?;
						Ok(RetVal::Converging(0))
					},
					2006 => {
						// Transfer tokens
						let from: AccountId = env.ext().caller().clone();
						let to: AccountId = env.read_as()?;
						let value: Balance = env.read_as()?;

						let origin = RawOrigin::Signed(from);
						let err_code = match Currencies::transfer(origin.into(), to, currency, value).is_ok() {
							true => 0,
							false => 2, // Err::InsufficientBalance
						};
						Ok(RetVal::Converging(err_code))
					},
					2007 => {
						// transfer_from
						// @dev: This is an UNSAFE method. Only whitelisted contracts can access it!

						let contract: AccountId = env.ext().address().clone();
						//@todo: Verify that the contract is authorised to do this operation

						let from: AccountId = env.read_as()?;
						let to: AccountId = env.read_as()?;
						let value: Balance = env.read_as()?;

						let origin = RawOrigin::Signed(from);
						let err_code = match Currencies::transfer(origin.into(), to, currency, value).is_ok() {
							true => 0,
							false => 2, // Err::InsufficientBalance
						};
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
