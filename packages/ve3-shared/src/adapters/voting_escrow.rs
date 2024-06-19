use crate::{
  error::SharedError,
  extensions::asset_ext::AssetExt,
  helpers::time::Time,
  msgs_voting_escrow::{ExecuteMsg, QueryMsg, VotingPowerFixedResponse, VotingPowerResponse},
};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, QuerierWrapper, StdResult};
use cw_asset::Asset;

pub struct VotingEscrow(pub Addr);

impl VotingEscrow {
  pub fn query_total_vp(
    &self,
    querier: &QuerierWrapper,
    time: Option<Time>,
  ) -> StdResult<VotingPowerResponse> {
    querier.query_wasm_smart(
      self.0.clone(),
      &QueryMsg::TotalVamp {
        time,
      },
    )
  }

  pub fn query_total_fixed(
    &self,
    querier: &QuerierWrapper,
    time: Option<Time>,
  ) -> StdResult<VotingPowerFixedResponse> {
    querier.query_wasm_smart(
      self.0.clone(),
      &QueryMsg::TotalFixed {
        time,
      },
    )
  }

  pub fn create_permanent_lock_msg(
    &self,
    asset: Asset,
    recipient: Option<String>,
  ) -> Result<CosmosMsg, SharedError> {
    asset.send_or_execute_msg(
      self.0.to_string(),
      to_json_binary(&ExecuteMsg::CreateLock {
        time: None,
        recipient,
      })?,
    )
  }

  pub fn create_extend_lock_amount_msg(
    &self,
    asset: Asset,
    token_id: String,
  ) -> Result<CosmosMsg, SharedError> {
    asset.send_or_execute_msg(
      self.0.to_string(),
      to_json_binary(&ExecuteMsg::ExtendLockAmount {
        token_id,
      })?,
    )
  }
}
