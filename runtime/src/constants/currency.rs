use primitives::{Balance, TokenId};

pub const NATIVE_TOKEN: TokenId = TokenId::Laguna;

// 1 Unit of LAGUNAS consists of 10^12 LAGUNA
pub const LAGUNAS: Balance = 1_000_000_000_000;
pub const MILLI_LAGUNAS: Balance = 1_000_000_000;
pub const MICRO_LAGUNAS: Balance = 1_000_000;

// LAGUNAS is assumed to be the $DOLLAR in the laguna-chain
pub const DOLLARS: Balance = LAGUNAS;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLI_CENTS: Balance = CENTS / 1_000;
