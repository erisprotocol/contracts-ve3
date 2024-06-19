// AT = Address Type
pub const AT_DELEGATION_CONTROLLER: &str = "DELEGATION_CONTROLLER";
pub const AT_ASSET_WHITELIST_CONTROLLER: &str = "ASSET_WHITELIST_CONTROLLER";
pub const AT_BRIBE_WHITELIST_CONTROLLER: &str = "BRIBE_WHITELIST_CONTROLLER";
pub const AT_VE_GUARDIAN: &str = "VE_GUARDIAN";

pub const AT_VOTING_ESCROW: &str = "VOTING_ESCROW";
pub const AT_ASSET_GAUGE: &str = "ASSET_GAUGE";
pub const AT_BRIBE_MANAGER: &str = "BRIBE_MANAGER";

pub const AT_FREE_BRIBES: &str = "FREE_BRIBES";
pub const AT_TAKE_RECIPIENT: &str = "TAKE_RECIPIENT";
pub const AT_FEE_COLLECTOR: &str = "FEE_COLLECTOR";
pub const AT_TEAM_WALLET: &str = "TEAM_WALLET";
pub const AT_MINT_PROXY: &str = "MINT_PROXY";

const AT_ASSET_STAKING: &str = "ASSET_STAKING";
pub fn at_asset_staking(gauge: &str) -> String {
  format!("{AT_ASSET_STAKING}__{gauge}")
}
const AT_CONNECTOR: &str = "CONNECTOR";
pub fn at_connector(gauge: &str) -> String {
  format!("{AT_CONNECTOR}__{gauge}")
}

pub const DEFAULT_LIMIT: u32 = 30;
pub const DEFAULT_PERIODS_LIMIT: u64 = 20;
pub const MAX_LIMIT: u32 = 100;

pub const SECONDS_PER_YEAR: u64 = 60 * 60 * 24 * 365;

// VOTING ESCROW
// Seconds in one week. It is intended for period number calculation.
// mainnet: 7 * 86400
// testnet: 60 * 60
pub const SECONDS_PER_WEEK: u64 = 7 * 86400;
/// Seconds in 2 years which is the maximum lock period.
pub const MAX_LOCK_TIME: u64 = 2 * 365 * 86400;
// 2 years (104 weeks)
pub const MAX_LOCK_PERIODS: u64 = 104;
/// Funds need to be at least locked for 3 weeks.
pub const MIN_LOCK_PERIODS: u64 = 1;
/// Monday, October 31, 2022 12:00:00 AM
pub const EPOCH_START: u64 = 1667174400;
