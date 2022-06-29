use codec::Encode;
use frame_support::dispatch::{DispatchInfo, Weight};

pub fn info_from_weight(w: Weight) -> DispatchInfo {
	// pays_fee: Pays::Yes -- class: DispatchClass::Normal
	DispatchInfo { weight: w, ..Default::default() }
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::*;
	use frame_support::{
		assert_ok,
		dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
		sp_runtime::traits::SignedExtension,
		weights::{DispatchClass, Pays},
	};
	use laguna_runtime::{constants::LAGUNAS, Currencies, FluentFee, Origin, TransactionPayment};
	use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};

	#[test]
	fn test_basic_fee_payout() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				// prepare a call
				let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
					to: BOB,
					currency_id: NATIVE_CURRENCY_ID,
					balance: 1 * LAGUNAS,
				});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

				let pre_validate_amount = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

				// validate will trigger all of the SignedExtension, and since one of them is
				// consumed by `TransactionPayment --> OnchargeTransaction --> FluentFee`
				// we can test fee chargin logic by calling validate once
				assert_ok!(ChargeTransactionPayment::<Runtime>::from(0)
					.validate(&ALICE, &call, &info, len));

				// calculate actual fee with all the parameter including base_fee, length_fee and
				// byte_multiplier etc.
				let fee = TransactionPayment::compute_actual_fee(
					len as u32,
					&info,
					&PostDispatchInfo { actual_weight: Some(info.weight), pays_fee: Pays::Yes },
					0,
				);

				let post_validate_amount = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

				assert_eq!(pre_validate_amount, post_validate_amount + fee);

				assert_ok!(call.dispatch(Origin::signed(ALICE)));

				assert_eq!(
					Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID),
					10 * LAGUNAS - 1 * LAGUNAS - fee
				);
			});
	}
}
