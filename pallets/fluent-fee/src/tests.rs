//! Unit test for the fluent-fee pallet

#![cfg(test)]

use super::*;

use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};



#[test]
fn add_fee_source() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(FluentFee::add_currency_to_fee_source(
            Origin::root(),
            NATIVE_CURRENCY_ID,
            FeeRatePoint { base: 10, point: 9 }
        ));
    });
}
