use cosmwasm_std::Decimal;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MAX_FEE: Decimal = Decimal::percent(20);
pub const RELAVANT_EXCHANGE_RATES: usize = 3;
