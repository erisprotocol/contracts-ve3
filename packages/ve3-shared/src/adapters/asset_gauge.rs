use crate::{
  error::SharedError,
  extensions::asset_ext::AssetExt,
  helpers::time::Times,
  msgs_asset_gauge::{
    ExecuteMsg, LastDistributionPeriodResponse, QueryMsg, UserFirstParticipationResponse,
    UserSharesResponse,
  },
};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, QuerierWrapper, StdResult};
use cw_asset::Asset;

pub struct AssetGauge(pub Addr);

impl AssetGauge {
  pub fn query_user_shares(
    &self,
    querier: &QuerierWrapper,
    user: Addr,
    times: Option<Times>,
  ) -> StdResult<UserSharesResponse> {
    querier.query_wasm_smart(
      self.0.clone(),
      &QueryMsg::UserShares {
        user,
        times,
      },
    )
  }

  pub fn query_first_participation(
    &self,
    querier: &QuerierWrapper,
    user: Addr,
  ) -> StdResult<UserFirstParticipationResponse> {
    querier.query_wasm_smart(
      self.0.clone(),
      &QueryMsg::UserFirstParticipation {
        user,
      },
    )
  }

  pub fn query_last_distribution_period(
    &self,
    querier: &QuerierWrapper,
  ) -> StdResult<LastDistributionPeriodResponse> {
    querier.query_wasm_smart(self.0.clone(), &QueryMsg::LastDistributionPeriod {})
  }

  pub fn add_rebase_msg(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
    asset.send_or_execute_msg(self.0.to_string(), to_json_binary(&ExecuteMsg::AddRebase {})?)
  }
}
