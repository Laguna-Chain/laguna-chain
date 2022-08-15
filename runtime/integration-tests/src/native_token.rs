//! native-token test
//!
//! test basic utilities of a multi-token system, where native or added token are abled to be
//! consumed by free, reserved and locked balance accounting system

#[cfg(test)]
mod tests {

	use crate::*;
	use frame_support::assert_ok;
	use laguna_runtime::{constants::LAGUNAS, Currencies};
	use orml_traits::{MultiCurrency, MultiCurrencyExtended, MultiReservableCurrency};

	#[test]
	fn transfer_native() {
		ExtBuilder::default()
			.balances(vec![
				(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS),
				(BOB, NATIVE_CURRENCY_ID, 10 * LAGUNAS),
			])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * LAGUNAS);

				let bob_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &BOB);
				assert_eq!(bob_init, 10 * LAGUNAS);

				assert_ok!(<Currencies as MultiCurrency<_>>::transfer(
					NATIVE_CURRENCY_ID,
					&ALICE,
					&BOB,
					LAGUNAS,
				));

				let alice_after =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_after, 9 * LAGUNAS);

				let bob_after =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &BOB);
				assert_eq!(bob_after, 11 * LAGUNAS);
			});
	}

	#[test]
	fn transfer_fee_token() {
		const FEE_TOKEN_ID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);
		ExtBuilder::default()
			.balances(vec![(ALICE, FEE_TOKEN_ID, 10 * LAGUNAS), (BOB, FEE_TOKEN_ID, 10 * LAGUNAS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &ALICE);
				assert_eq!(alice_init, 10 * LAGUNAS);

				let bob_init = <Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &BOB);
				assert_eq!(bob_init, 10 * LAGUNAS);

				assert_ok!(<Currencies as MultiCurrency<_>>::transfer(
					FEE_TOKEN_ID,
					&ALICE,
					&BOB,
					LAGUNAS,
				));

				let alice_after =
					<Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &ALICE);
				assert_eq!(alice_after, 9 * LAGUNAS);

				let bob_after = <Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &BOB);
				assert_eq!(bob_after, 11 * LAGUNAS);
			});
	}

	#[test]
	fn set_token_balance() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * LAGUNAS);

				assert_ok!(
					Currencies::update_balance(NATIVE_CURRENCY_ID, &ALICE, LAGUNAS as i128,)
				);

				let alice_after =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_after, 11 * LAGUNAS);
			});
	}

	#[test]
	fn reserve_balance() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * LAGUNAS);

				assert_ok!(Currencies::reserve(NATIVE_CURRENCY_ID, &ALICE, LAGUNAS,));

				let alice_free =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_free, 9 * LAGUNAS);

				let alice_reserved = <Currencies as MultiReservableCurrency<_>>::reserved_balance(
					NATIVE_CURRENCY_ID,
					&ALICE,
				);
				assert_eq!(alice_reserved, LAGUNAS);
			});
	}

	#[test]
	fn slash_balance() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * LAGUNAS);

				// should return 0 if full target amount is slashed
				assert_eq!(Currencies::slash(NATIVE_CURRENCY_ID, &ALICE, LAGUNAS), 0);
				let alice_free =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_free, 9 * LAGUNAS);

				assert_ok!(<Currencies as MultiReservableCurrency<_>>::reserve(
					NATIVE_CURRENCY_ID,
					&ALICE,
					LAGUNAS,
				));
				assert_eq!(
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE),
					8 * LAGUNAS
				);

				assert_eq!(
					<Currencies as MultiReservableCurrency<_>>::slash_reserved(
						NATIVE_CURRENCY_ID,
						&ALICE,
						LAGUNAS
					),
					0
				);
				assert_eq!(
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE),
					8 * LAGUNAS
				);

				assert_eq!(
					<Currencies as MultiReservableCurrency<_>>::reserved_balance(
						NATIVE_CURRENCY_ID,
						&ALICE
					),
					0
				);

				// not enough reserved to be slashed, balance should be unchanged
				assert!(
					<Currencies as MultiReservableCurrency<_>>::slash_reserved(
						NATIVE_CURRENCY_ID,
						&ALICE,
						LAGUNAS
					) != 0
				);
				assert_eq!(
					<Currencies as MultiReservableCurrency<_>>::reserved_balance(
						NATIVE_CURRENCY_ID,
						&ALICE
					),
					0
				);
			});
	}

	// TODO: add lock test for features requiring asset to have characteristic of liquidity
}
