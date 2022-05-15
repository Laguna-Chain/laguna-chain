use crate::{mock::*, AccountIdOf};
use frame_support::traits::{fungible, fungibles};
use orml_traits::{BasicCurrency, MultiCurrency};

#[test]
fn test_total_supply() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, NativeCurrencyId::get(), UNIT),
			(BOB, NativeCurrencyId::get(), UNIT),
		])
		.build()
		.execute_with(|| {
			dbg!(<Currencies as fungibles::Inspect<AccountIdOf<Runtime>>>::balance(
				NativeCurrencyId::get(),
				&ALICE
			));
		});
}
