use cosmwasm_std::Decimal;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Scaling denominator for commission
pub const COMMISSION_DENOM: u64 = 10000u64;
// 0.003 %
pub const COMMISSION_DEFAULT: u64 = 30u64;

pub const DEFAULT_MAX_SPREAD: Decimal = Decimal::percent(10);
pub const DEFAULT_SLIPPAGE: Decimal = Decimal::percent(10);

pub const OPTIMAL_SWAP_ITERATIONS: u64 = 16;
