use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, QuerierWrapper, StdResult};
use serde::Deserialize;

pub struct ErisHub<'a>(pub &'a Addr);

#[cw_serde]
pub enum QueryMsg {
  State {},
}

#[derive(Deserialize)]
pub struct StateResponse {
  /// The exchange rate between ustake and uluna, in terms of uluna per ustake
  pub exchange_rate: Decimal,
}

impl<'a> ErisHub<'a> {
  pub fn query_exchange_rate(&self, querier: &QuerierWrapper) -> StdResult<Decimal> {
    let result: StateResponse =
      querier.query_wasm_smart(self.0.to_string(), &QueryMsg::State {})?;
    Ok(result.exchange_rate)
  }
}
