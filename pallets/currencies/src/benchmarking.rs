use super::*;
use crate::Pallet as Currencies;
#[allow(unused)]
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
	dispatch::{Encode, HasCompact},
	traits::tokens::currency::Currency,
};
use frame_system::{Config as SysConfig, Origin, RawOrigin};
use pallet_contract_asset_registry;
use pallet_contracts::chain_extension::UncheckedFrom;
use primitives::{AccountId, CurrencyId, TokenId};
use sp_core::{Bytes, U256};
use std::str::FromStr;

const SEED: u32 = 0;
const INIT_BALANCE: u128 = 1000_000;

fn create_token<T, U>(
	owner: T::AccountId,
	tkn_name: &str,
	tkn_symbol: &str,
	init_amount: U,
) -> AccountId
where
	T: pallet_contracts::Config + pallet_contract_asset_registry::Config,
	<<<T as pallet_contracts::Config>::Currency as Currency<<T as SysConfig>::AccountId>>::Balance as HasCompact>::Type: Clone + std::cmp::Eq + PartialEq + std::fmt::Debug + TypeInfo + Encode,
	<T as pallet_contracts::Config>::Currency: Currency<<T as SysConfig>::AccountId, Balance = u128>,
	<T as frame_system::Config>::AccountId: UncheckedFrom<<T as SysConfig>::Hash> + AsRef<[u8]>,
// <T as SysConfig>::Origin: From<RawOrigin<AccountId32>>,
	U256: From<U>,
{
	let blob = std::fs::read(
		"../../runtime/integration-tests/contracts-data/solidity/token/dist/DemoToken.wasm",
	)
	.expect("unable to read contract");

	let mut sel_constuctor = Bytes::from_str("0x835a15cb")
		.map(|v| v.to_vec())
		.expect("unable to parse selector");

	sel_constuctor.append(&mut tkn_name.encode());
	sel_constuctor.append(&mut tkn_symbol.encode());
	sel_constuctor.append(&mut U256::from(init_amount).encode());

	pallet_contracts::Pallet::<T>::instantiate_with_code(
		Origin::Signed(owner).into(),
		0,
		<T as pallet_contract_asset_registry::Config>::MaxGas::get(),
		None, /* if not specified, it's allowed to charge the max amount of free balance of the
		       * creator */
		blob,
		sel_constuctor,
		vec![],
	)
	.expect("Error instantiating the code");

	let evts = frame_system::Pallet::<T>::events();
	let Event = <T as frame_system::Config>::Event;
	let deployed = evts
		.iter()
		.rev()
		.find_map(|rec| {
			if let Event::Contracts(pallet_contracts::Event::Instantiated {
				deployer: _,
				contract,
			}) = &rec.event
			{
				Some(contract)
			} else {
				None
			}
		})
		.expect("unable to find deployed contract");

	deployed.clone()
}

benchmarks! {
	where_clause {
		where
		T: crate::Config,
		T::MultiCurrency: MultiCurrency<T::AccountId, CurrencyId = CurrencyId, Balance = u128>
	}
	// Transfer native tokens, presumably it is less expensive in terms
	// of gas than ERC20 token transfers which are contract calls
	transfer {
		let caller: T::AccountId = whitelisted_caller();
		let a in 0..100000;
		let recipient: T::AccountId = account("recipient", 0, SEED);

		// Deposit some free balance for the caller
		<T::MultiCurrency as MultiCurrency<T::AccountId>>::deposit(
			T::NativeCurrencyId::get(),
			&caller,
			INIT_BALANCE.clone(),
		)?;
		// Currency to transfer
		let currency_id: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
	}: _(RawOrigin::Signed(caller.clone()), recipient.clone(), currency_id.clone(), a.clone().into())
	verify {
		assert_eq!(<T::MultiCurrency as MultiCurrency<T::AccountId>>::free_balance(currency_id.clone(), &recipient), a.into());
		assert_eq!(<T::MultiCurrency as MultiCurrency<T::AccountId>>::free_balance(currency_id.clone(), &caller), INIT_BALANCE - u128::from(a));
	}

	impl_benchmark_test_suite!(Currencies, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}
