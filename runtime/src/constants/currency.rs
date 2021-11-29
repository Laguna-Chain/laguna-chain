use primitives::{Balance, TokenId};

pub const NATIVE_TOKEN: TokenId = TokenId::Hydro;

// 1 Unit of HYDROS consists of 10^12 HYDRO
pub const HYDROS: Balance = 1_000_000_000_000;
pub const MILLI_HYDRO: Balance = 1_000_000_000;
pub const MICRO_HYDRO: Balance = 1_000_000;

// HYDROS is assumed to be the $DOLLAR in the hydro-chain
pub const DOLLARS: Balance = HYDROS;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLI_CENTS: Balance = CENTS / 1_000;
