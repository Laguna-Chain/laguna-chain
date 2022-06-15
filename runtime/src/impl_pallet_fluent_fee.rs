use core::marker::PhantomData;

use crate::{Call, Currencies, Event, FluentFee, Runtime, Tokens};
use frame_support::{
	pallet_prelude::{DispatchError, InvalidTransaction, TransactionValidityError},
	parameter_types,
	traits::WithdrawReasons,
};
use orml_traits::{LockIdentifier, MultiCurrency};
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource};

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Tokens;

	type Call = Call;

	type FeeSource = StaticImpl;

	type FeeMeasure = StaticImpl;

	type FeeDispatch = StaticImpl;

	type TreasuryAccount = TreasuryAccount;

	// type NativeCurrencyId = NativeCurrencyId;

	type LockId = LockId;
}

pub const TREASURY_ACCOUNT: AccountId = AccountId::new([9u8; 32]);
pub const LOCK_ID: LockIdentifier = *b"1       ";

parameter_types! {
	pub const TreasuryAccount: AccountId = TREASURY_ACCOUNT;
	pub const LockId: LockIdentifier = LOCK_ID;

}

pub struct StaticImpl;

impl FeeSource for StaticImpl {
	type AssetId = CurrencyId;

	type Balance = Balance;

	fn accepted(id: &Self::AssetId) -> Result<(), DispatchError> {
		if let CurrencyId::NativeToken(TokenId::FeeToken | TokenId::Laguna) = id {
			Ok(())
		} else if FluentFee::accepted_assets(&id) {
			Ok(())
		} else {
			Err(DispatchError::Other("InvalidFeeSource: Unlisted"))
		}
	}

	fn listing_asset(id: &Self::AssetId) -> Result<(), DispatchError> {
		let staked_amount = FluentFee::total_staked(id);
		let total_supply = Tokens::total_issuance(id);

		if (staked_amount * 100 / total_supply) < 30 {
			Err(DispatchError::Other("InvalidFeeSource: Ineligible"))
		} else {
			pallet_fluent_fee::AcceptedAssets::<Runtime>::insert(&id, true);
			Ok(())
		}
	}

	fn denounce_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
	}

	fn disable_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
	}
}

impl FeeMeasure for StaticImpl {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, TransactionValidityError> {
		match id {
			CurrencyId::NativeToken(TokenId::Laguna) => Ok(balance),

			// demo 5% reduction
			CurrencyId::NativeToken(TokenId::FeeToken) =>
				Ok(balance.saturating_mul(95).saturating_div(100)),
			_ => Err(InvalidTransaction::Payment.into()),
		}
	}
}

impl FeeDispatch<Runtime> for StaticImpl {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
		native_balance: &Self::Balance,
		reason: &WithdrawReasons,
	) -> Result<(), DispatchError> {
		let current_user_balance = FluentFee::treasury_balance_per_account(account);
		if current_user_balance >= *native_balance {
			// return Err(traits::fee::InvalidFeeSource::Insufficient)
			Tokens::withdraw(*id, &TREASURY_ACCOUNT, *native_balance)?;
			// Let the treasury pay the fee on behalf of the user if they have already prepaid
			let new_user_balance = current_user_balance - native_balance;
			pallet_fluent_fee::TreasuryBalancePerAccount::<Runtime>::insert(
				&account,
				new_user_balance,
			);
		} else {
			Tokens::withdraw(*id, &account, *balance)?;
		}

		Ok(())
	}

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &sp_runtime::traits::PostDispatchInfoOf<<Runtime as frame_system::Config>::Call>,
	) -> Result<(), traits::fee::InvalidFeeSource> {
		Ok(())
	}
}
