// AT = Address Type
pub const AT_CONNECTOR: &str = "CONNECTOR";
pub const AT_DELEGATION_CONTROLLER: &str = "DELEGATION_CONTROLLER";
pub const AT_ASSET_WHITELIST_CONTROLLER: &str = "ASSET_WHITELIST_CONTROLLER";
pub const AT_REWARD_DISTRIBUTION_CONTROLLER: &str = "REWARD_DISTRIBUTION_CONTROLLER";
pub const AT_LP_STAKING: &str = "LP_STAKING";
pub const AT_TAKE_RECIPIENT: &str = "TAKE_RECIPIENT";
pub const AT_VE_GUARDIAN: &str = "VE_GUARDIAN";

pub const DEFAULT_LIMIT: u32 = 10;
pub const DEFAULT_PERIODS_LIMIT: u64 = 20;
pub const MAX_LIMIT: u32 = 100;

pub const SECONDS_PER_YEAR: u64 = 60 * 60 * 24 * 365;

// VOTING ESCROW
// Seconds in one week. It is intended for period number calculation.
// mainnet: 7 * 86400
// testnet: 60 * 60
pub const WEEK: u64 = 7 * 86400;
/// Seconds in 2 years which is the maximum lock period.
pub const MAX_LOCK_TIME: u64 = 2 * 365 * 86400; // 2 years (104 weeks)
/// Funds need to be at least locked for 3 weeks.
pub const MIN_LOCK_PERIODS: u64 = 1;
/// Monday, October 31, 2022 12:00:00 AM
pub const EPOCH_START: u64 = 1667174400;
