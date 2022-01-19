#[cfg(test)]
mod tests {

    use crate::*;
    use frame_support::assert_ok;
    use hydro_runtime::{constants::HYDROS, Currencies, Origin};
    use orml_traits::MultiCurrency;

    #[test]
    fn transfer_native() {
        ExtBuilder::default()
            .balances(vec![
                (ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS),
                (BOB, NATIVE_CURRENCY_ID, 10 * HYDROS),
            ])
            .build()
            .execute_with(|| {
                let alice_init = Currencies::free_balance(NATIVE_CURRENCY_ID, &ALICE);
                assert_eq!(alice_init, 10 * HYDROS);

                let bob_init = Currencies::free_balance(NATIVE_CURRENCY_ID, &BOB);
                assert_eq!(bob_init, 10 * HYDROS);

                assert_ok!(Currencies::transfer(
                    Origin::signed(ALICE.into()),
                    BOB.into(),
                    NATIVE_CURRENCY_ID,
                    1 * HYDROS,
                ));

                let alice_after = Currencies::free_balance(NATIVE_CURRENCY_ID, &ALICE);
                assert_eq!(alice_after, 9 * HYDROS);

                let bob_after = Currencies::free_balance(NATIVE_CURRENCY_ID, &BOB);
                assert_eq!(bob_after, 11 * HYDROS);
            });
    }

    #[test]
    fn transfer_fee_token() {
        const FEE_TOKEN_ID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);
        ExtBuilder::default()
            .balances(vec![
                (ALICE, FEE_TOKEN_ID, 10 * HYDROS),
                (BOB, FEE_TOKEN_ID, 10 * HYDROS),
            ])
            .build()
            .execute_with(|| {
                let alice_init = Currencies::free_balance(FEE_TOKEN_ID, &ALICE);
                assert_eq!(alice_init, 10 * HYDROS);

                let bob_init = Currencies::free_balance(FEE_TOKEN_ID, &BOB);
                assert_eq!(bob_init, 10 * HYDROS);

                assert_ok!(Currencies::transfer(
                    Origin::signed(ALICE.into()),
                    BOB.into(),
                    FEE_TOKEN_ID,
                    1 * HYDROS,
                ));

                let alice_after = Currencies::free_balance(FEE_TOKEN_ID, &ALICE);
                assert_eq!(alice_after, 9 * HYDROS);

                let bob_after = Currencies::free_balance(FEE_TOKEN_ID, &BOB);
                assert_eq!(bob_after, 11 * HYDROS);
            });
    }

    #[test]
    fn set_token_balance() {
        ExtBuilder::default()
            .balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * HYDROS)])
            .build()
            .execute_with(|| {
                let alice_init = Currencies::free_balance(NATIVE_CURRENCY_ID, &ALICE);
                assert_eq!(alice_init, 10 * HYDROS);

                assert_ok!(Currencies::update_balance(
                    Origin::root(),
                    ALICE.into(),
                    NATIVE_CURRENCY_ID,
                    (1 * HYDROS) as i128,
                ));

                let alice_after = Currencies::free_balance(NATIVE_CURRENCY_ID, &ALICE);
                assert_eq!(alice_after, 11 * HYDROS);
            });
    }
}
