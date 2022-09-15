use primitives::BlockNumber;

// block time of 6 seconds
pub const MILLISECONDS_PER_BLOCK: BlockNumber = 6_000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECONDS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;
