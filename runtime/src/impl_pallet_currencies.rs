use frame_support::{parameter_types, traits::Currency};
use orml_tokens::CurrencyAdapter;
use primitives::{AccountId, Amount, BlockNumber, CurrencyId, TokenId};
use sp_runtime::traits::{Convert, ConvertInto};

use crate::{ContractAssetsRegistry, Runtime, Tokens};

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
}

impl pallet_currencies::Config for Runtime {
	type NativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type ContractAssets = ContractAssetsRegistry;
	type ConvertIntoAccountId = ConvertInto;
}

struct AddressConvert;

impl Convert<[u8; 32], AccountId> for AddressConvert {
	fn convert(a: [u8; 32]) -> AccountId {
		a.into()
	}
}
