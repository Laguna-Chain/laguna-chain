use primitives::{Balance, CurrencyId, TokenId};

pub const LAGUNA_TOKEN: TokenId = TokenId::Laguna;
pub const LAGUNA_NATIVE_CURRENCY: CurrencyId = CurrencyId::NativeToken(LAGUNA_TOKEN);

// 1 Unit of LAGUNAS consists of 10^18 LAGUNA
pub const LAGUNAS: Balance = 10_u128.pow(18);
pub const MILLI_LAGUNAS: Balance = 10_u128.pow(15);
pub const MICRO_LAGUNAS: Balance = 10_u128.pow(12);
