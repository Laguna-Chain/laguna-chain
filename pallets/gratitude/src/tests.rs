use super::{mock, mock::*, *};
use frame_support::*;
use orml_tokens;

#[test]
fn tip_ok() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// validate initial state
		assert_eq!(Tokens::accounts(BOB, GRATITUDE_CURRENCY_ID).free, 20000);

		assert_ok!(
			Gratitude::tip(Origin::signed(BOB), 5000, b"tip1".to_vec().try_into().unwrap(),)
		);

		// check the event
		System::assert_last_event(mock::Event::Gratitude(crate::Event::GratitudeAccepted {
			from: BOB,
			amount: 5000,
			reason: b"tip1".to_vec().try_into().unwrap(),
		}));
		// check the new GratitudeTrail
		assert_eq!(
			GratitudeTrail::<Runtime>::get(BOB, 1),
			Some((5000, b"tip1".to_vec().try_into().unwrap()))
		);
		// check updated HGRAD balance
		assert_eq!(Tokens::accounts(BOB, GRATITUDE_CURRENCY_ID).free, 15000);
	});
}

#[test]
fn tip_insufficient_funds() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			Gratitude::tip(
				Origin::signed(BOB),
				500000000000000,
				b"invalid_tip".to_vec().try_into().unwrap(),
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}
