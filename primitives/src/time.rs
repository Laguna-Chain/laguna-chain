//! # Time module
//!
//! thie module implements various time conversion helper based on BlockNumber and milliseconds per
//! block

use super::BlockNumber;

// time measurement based on adjustable blocktime
// allow conversion based on runtime specified milliseconds per block
// derived from substrate-node-template

pub const fn minutes(millisec_per_block: u64) -> BlockNumber {
	60_000 / (millisec_per_block as BlockNumber)
}

pub const fn hours(millisec_per_block: u64) -> BlockNumber {
	minutes(millisec_per_block) * 60
}

pub const fn days(millisec_per_block: u64) -> BlockNumber {
	hours(millisec_per_block) * 24
}
