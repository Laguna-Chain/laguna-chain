//! native-token test
//!
//! test basic utilities of a multi-token system, where native or added token are abled to be
//! consumed by free, reserved and locked balance accounting system

#[cfg(test)]
mod tests {

	use crate::*;
	use frame_support::assert_ok;
	use hydro_runtime::{constants::HYDROS, Currencies, Origin};
	use orml_traits::{
		MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
	};

	#[test]
	fn transfer_native() {
		ExtBuilder::default()
			.balances(vec![
				(ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS),
				(BOB, NATIVE_CURRENCY_ID, 10 * HYDROS),
			])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * HYDROS);

				let bob_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &BOB);
				assert_eq!(bob_init, 10 * HYDROS);

				assert_ok!(<Currencies as MultiCurrency<_>>::transfer(
					NATIVE_CURRENCY_ID,
					&ALICE,
					&BOB,
					1 * HYDROS,
				));

				let alice_after =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_after, 9 * HYDROS);

				let bob_after =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &BOB);
				assert_eq!(bob_after, 11 * HYDROS);
			});
	}

	#[test]
	fn transfer_fee_token() {
		const FEE_TOKEN_ID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);
		ExtBuilder::default()
			.balances(vec![(ALICE, FEE_TOKEN_ID, 10 * HYDROS), (BOB, FEE_TOKEN_ID, 10 * HYDROS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &ALICE);
				assert_eq!(alice_init, 10 * HYDROS);

				let bob_init = <Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &BOB);
				assert_eq!(bob_init, 10 * HYDROS);

				assert_ok!(<Currencies as MultiCurrency<_>>::transfer(
					FEE_TOKEN_ID,
					&ALICE,
					&BOB,
					1 * HYDROS,
				));

				let alice_after =
					<Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &ALICE);
				assert_eq!(alice_after, 9 * HYDROS);

				let bob_after = <Currencies as MultiCurrency<_>>::free_balance(FEE_TOKEN_ID, &BOB);
				assert_eq!(bob_after, 11 * HYDROS);
			});
	}

	#[test]
	fn set_token_balance() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * HYDROS);

				assert_ok!(Currencies::update_balance(
					NATIVE_CURRENCY_ID,
					&ALICE,
					(1 * HYDROS) as i128,
				));

				let alice_after =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_after, 11 * HYDROS);
			});
	}

	#[test]
	fn reserve_balance() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * HYDROS);

				assert_ok!(Currencies::reserve(NATIVE_CURRENCY_ID, &ALICE, 1 * HYDROS,));

				let alice_free =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_free, 9 * HYDROS);

				let alice_reserved = <Currencies as MultiReservableCurrency<_>>::reserved_balance(
					NATIVE_CURRENCY_ID,
					&ALICE,
				);
				assert_eq!(alice_reserved, 1 * HYDROS);
			});
	}

	#[test]
	fn slash_balance() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS)])
			.build()
			.execute_with(|| {
				let alice_init =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_init, 10 * HYDROS);

				// should return 0 if full target amount is slashed
				assert_eq!(Currencies::slash(NATIVE_CURRENCY_ID, &ALICE, HYDROS), 0);
				let alice_free =
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE);
				assert_eq!(alice_free, 9 * HYDROS);

				assert_ok!(<Currencies as MultiReservableCurrency<_>>::reserve(
					NATIVE_CURRENCY_ID,
					&ALICE,
					1 * HYDROS,
				));
				assert_eq!(
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE),
					8 * HYDROS
				);

				assert_eq!(
					<Currencies as MultiReservableCurrency<_>>::slash_reserved(
						NATIVE_CURRENCY_ID,
						&ALICE,
						HYDROS
					),
					0
				);
				assert_eq!(
					<Currencies as MultiCurrency<_>>::free_balance(NATIVE_CURRENCY_ID, &ALICE),
					8 * HYDROS
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
						HYDROS
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
