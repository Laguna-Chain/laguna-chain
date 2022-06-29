use crate::{Call, ContractAssetsRegistry, Currencies, Event, FluentFee, Runtime, Tokens};
use frame_support::{
	pallet_prelude::{DispatchError, InvalidTransaction, TransactionValidityError},
	parameter_types,
	traits::WithdrawReasons,
};
use orml_traits::{LockIdentifier, MultiCurrency};
use pallet_contract_asset_registry::TokenAccess;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use sp_core::U256;
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
pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
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
		match id {
			CurrencyId::Erc20(_) => {
				pallet_fluent_fee::AcceptedAssets::<Runtime>::insert(&id, true);
				Ok(())
			},
			CurrencyId::NativeToken(_) => {
				let staked_amount = FluentFee::total_staked(id);
				let total_supply = Tokens::total_issuance(id);

				if (staked_amount * 100 / total_supply) < 30 {
					Err(DispatchError::Other("InvalidFeeSource: Ineligible"))
				} else {
					pallet_fluent_fee::AcceptedAssets::<Runtime>::insert(&id, true);
					Ok(())
				}
			},
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
			CurrencyId::Erc20(_) => Ok(balance.saturating_mul(70).saturating_div(100)),
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
		// Let the treasury pay the fee on behalf of the user if they have already prepaid
		if current_user_balance >= *native_balance {
			Tokens::withdraw(NATIVE_CURRENCY_ID, &TREASURY_ACCOUNT, *native_balance)?;
			let new_user_balance = current_user_balance - native_balance;
			pallet_fluent_fee::TreasuryBalancePerAccount::<Runtime>::insert(
				&account,
				new_user_balance,
			);
		}
		// If there doesn't exist enough balance for the user in the treasury make the user directly
		// pay for the transaction.
		else {
			match *id {
				CurrencyId::NativeToken(_) => Tokens::withdraw(*id, &account, *balance)?,
				CurrencyId::Erc20(asset_address) => ContractAssetsRegistry::transfer(
					asset_address.into(),
					account.clone(),
					TREASURY_ACCOUNT,
					U256::from(*balance),
				)
				.map(|_| ())
				.map_err(|_| DispatchError::CannotLookup)?,
			}
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
