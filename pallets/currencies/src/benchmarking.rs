#![cfg(feature = "runtime-benchmarks")]

use super::*;

#[allow(unused)]
use crate::Pallet as Currencies;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use primitives::{CurrencyId, TokenId};
use sp_core::U256;

benchmarks! {
	transfer_native {
		let currency_id = CurrencyId::NativeToken(TokenId::Laguna);
		let caller: T::AccountId = whitelisted_caller();
		let to: T::AccountId = whitelisted_caller();
		let balance = 1000u32;
	}: _(RawOrigin::Signed(caller.clone()), to, currency_id, amount)
	verify {
	}



	impl_benchmark_test_suite!(Currencies, crate::mock::ExtBuilder::default().build(), crate::mock::Runtime);
}

fn create_token<T>(owner: AccountId, tkn_name: &str, tkn_symbol: &str, init_amount: T) -> AccountId
where
	U256: From<T>,
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

	assert_ok!(Contracts::instantiate_with_code(
		Origin::signed(owner),
		0,
		MaxGas::get(),
		None, /* if not specified, it's allowed to charge the max amount of free balance of the
		       * creator */
		blob,
		sel_constuctor,
		vec![]
	));

	let evts = System::events();
	let deployed = evts
		.iter()
		.rev()
		.find_map(|rec| {
			if let MockEvent::Contracts(pallet_contracts::Event::Instantiated {
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
