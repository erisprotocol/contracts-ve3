use cosmwasm_std::Decimal;

// version info for migration info
pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
// pub const CREATE_REPLY_ID: u64 = 1;
pub const CLAIM_REWARD_ERROR_REPLY_ID: u64 = 2;

pub const MAX_OTC_DISCOUNT: Decimal = Decimal::percent(50);
pub const UFACTOR: u128 = 1000000;
