use super::pallet::Call as FluentFee;
use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			Convert, DispatchInfoOf, Dispatchable, One, PostDispatchInfoOf, SaturatedConversion,
			Saturating, SignedExtension, Zero,
		},
		transaction_validity::{
			TransactionPriority, TransactionValidity, TransactionValidityError, ValidTransaction,
		},
		FixedPointNumber, FixedPointOperand, FixedU128,
	},
	weights::{DispatchInfo, Pays, PostDispatchInfo},
};
use pallet_transaction_payment::{OnChargeTransaction, Pallet};
use scale_info::TypeInfo;

type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ChargeFeeSharingTransactionPayment<T: pallet_transaction_payment::Config>(
	#[codec(compact)] BalanceOf<T>,
);

impl<T: pallet_transaction_payment::Config> ChargeFeeSharingTransactionPayment<T>
where
	T::Call: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	BalanceOf<T>: Send + Sync + FixedPointOperand,
{
	/// utility constructor. Used only in client/factory code.
	pub fn from(fee: BalanceOf<T>) -> Self {
		Self(fee)
	}

	/// Returns the tip as being choosen by the transaction sender.
	pub fn tip(&self) -> BalanceOf<T> {
		self.0
	}

	// fn get_unit_weight_fee(&self, call: &T::Call) -> Option<BalanceOf<T>> {
	// 	match call {
	// 		FluentFee::fee_sharing_wrapper{..} => {},
	// 		_ => None,
	// 	}
	// }

	fn withdraw_fee(
		&self,
		who: &T::AccountId,
		call: &T::Call,
		info: &DispatchInfoOf<T::Call>,
		len: usize,
	) -> Result<
		(
			BalanceOf<T>,
			<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::LiquidityInfo,
		),
		TransactionValidityError,
	>{
		let tip = self.0;
		let call_fee = Pallet::<T>::compute_fee(len as u32, info, tip);
		// Get the unit weight equivalent fee at the current block
		let unit_weight_fee = Pallet::<T>::compute_fee_details(
			0,
			&DispatchInfo { pays_fee: Pays::Yes, weight: 1u64, class: /* Doesn't matter here unit weight fee is independent of the call type */DispatchClass::Normal },
			tip-tip,
		).inclusion_fee.unwrap().adjusted_weight_fee;

		let total_fee = call_fee + unit_weight_fee;
		<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::withdraw_fee(
			who, call, info, total_fee, tip,
		)
		.map(|i| (total_fee, i))
	}

	pub fn get_priority(
		info: &DispatchInfoOf<T::Call>,
		len: usize,
		tip: BalanceOf<T>,
		final_fee: BalanceOf<T>,
	) -> TransactionPriority {
		// Calculate how many such extrinsics we could fit into an empty block and take
		// the limitting factor.
		let max_block_weight = T::BlockWeights::get().max_block;
		let max_block_length = *T::BlockLength::get().max.get(info.class) as u64;

		let bounded_weight = info.weight.max(1).min(max_block_weight);
		let bounded_length = (len as u64).max(1).min(max_block_length);

		let max_tx_per_block_weight = max_block_weight / bounded_weight;
		let max_tx_per_block_length = max_block_length / bounded_length;
		// Given our current knowledge this value is going to be in a reasonable range - i.e.
		// less than 10^9 (2^30), so multiplying by the `tip` value is unlikely to overflow the
		// balance type. We still use saturating ops obviously, but the point is to end up with some
		// `priority` distribution instead of having all transactions saturate the priority.
		let max_tx_per_block = max_tx_per_block_length
			.min(max_tx_per_block_weight)
			.saturated_into::<BalanceOf<T>>();
		let max_reward = |val: BalanceOf<T>| val.saturating_mul(max_tx_per_block);

		// To distribute no-tip transactions a little bit, we increase the tip value by one.
		// This means that given two transactions without a tip, smaller one will be preferred.
		let tip = tip.saturating_add(One::one());
		let scaled_tip = max_reward(tip);

		match info.class {
			DispatchClass::Normal => {
				// For normal class we simply take the `tip_per_weight`.
				scaled_tip
			},
			DispatchClass::Mandatory => {
				// Mandatory extrinsics should be prohibited (e.g. by the [`CheckWeight`]
				// extensions), but just to be safe let's return the same priority as `Normal` here.
				scaled_tip
			},
			DispatchClass::Operational => {
				// A "virtual tip" value added to an `Operational` extrinsic.
				// This value should be kept high enough to allow `Operational` extrinsics
				// to get in even during congestion period, but at the same time low
				// enough to prevent a possible spam attack by sending invalid operational
				// extrinsics which push away regular transactions from the pool.
				let fee_multiplier = T::OperationalFeeMultiplier::get().saturated_into();
				let virtual_tip = final_fee.saturating_mul(fee_multiplier);
				let scaled_virtual_tip = max_reward(virtual_tip);

				scaled_tip.saturating_add(scaled_virtual_tip)
			},
		}
		.saturated_into::<TransactionPriority>()
	}
}

impl<T: pallet_transaction_payment::Config> sp_std::fmt::Debug
	for ChargeFeeSharingTransactionPayment<T>
{
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "ChargeFeeSharingTransactionPayment<{:?}>", self.0)
	}
	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T: pallet_transaction_payment::Config> SignedExtension
	for ChargeFeeSharingTransactionPayment<T>
where
	BalanceOf<T>: Send + Sync + From<u64> + FixedPointOperand,
	T::Call: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
	const IDENTIFIER: &'static str = "ChargeFeeSharingTransactionPayment";
	type AccountId = T::AccountId;
	type Call = T::Call;
	type AdditionalSigned = ();
	type Pre = (
		// tip
		BalanceOf<T>,
		// who paid the fee - this is an option to allow for a Default impl.
		Self::AccountId,
		// imbalance resulting from withdrawing the fee
		<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<
			T,
		>>::LiquidityInfo,
	);
	fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
		Ok(())
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> TransactionValidity {
		let (final_fee, _) = self.withdraw_fee(who, call, info, len)?;
		let tip = self.0;
		Ok(ValidTransaction {
			priority: Self::get_priority(info, len, tip, final_fee),
			..Default::default()
		})
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		let (_fee, imbalance) = self.withdraw_fee(who, call, info, len)?;
		Ok((self.0, who.clone(), imbalance))
	}

	fn post_dispatch(
		maybe_pre: Option<Self::Pre>,
		info: &DispatchInfoOf<Self::Call>,
		post_info: &PostDispatchInfoOf<Self::Call>,
		len: usize,
		_result: &DispatchResult,
	) -> Result<(), TransactionValidityError> {
		if let Some((tip, who, imbalance)) = maybe_pre {
			let actual_fee = Pallet::<T>::compute_actual_fee(len as u32, info, post_info, tip);
			T::OnChargeTransaction::correct_and_deposit_fee(
				&who, info, post_info, actual_fee, tip, imbalance,
			)?;
			// Pallet::<T>::deposit_event(Event::<T>::TransactionFeePaid { who, actual_fee, tip });
		}
		Ok(())
	}
}
