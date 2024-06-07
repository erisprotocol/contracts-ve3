use crate::{
  helpers::time::Times,
  msgs_asset_gauge::{QueryMsg, UserFirstParticipationResponse, UserSharesResponse},
};
use cosmwasm_std::{Addr, QuerierWrapper, StdResult};

pub struct AssetGauge(pub Addr);

impl AssetGauge {
  // pub fn claim_rewards_msg(&self, lp_tokens: Vec<AssetInfo>) -> Result<CosmosMsg, SharedError> {
  //     Ok(CosmosMsg::Wasm(WasmMsg::Execute {
  //         contract_addr: self.0.to_string(),
  //         msg: to_json_binary(&ExecuteMsg::ClaimRewardsMultiple(lp_tokens))?,
  //         funds: vec![],
  //     }))
  // }

  // pub fn deposit_msg(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
  //     match asset.info {
  //         cw_asset::AssetInfoBase::Native(native) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
  //             contract_addr: self.0.to_string(),
  //             msg: to_json_binary(&ExecuteMsg::Stake {
  //                 recipient: None,
  //             })?,
  //             funds: coins(asset.amount.u128(), native),
  //         })),
  //         cw_asset::AssetInfoBase::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
  //             contract_addr: contract_addr.to_string(),
  //             funds: vec![],
  //             msg: to_json_binary(&Cw20ExecuteMsg::Send {
  //                 contract: self.0.to_string(),
  //                 amount: asset.amount,
  //                 msg: to_json_binary(&Cw20HookMsg::Stake {
  //                     recipient: None,
  //                 })?,
  //             })?,
  //         })),
  //         _ => Err(SharedError::NotSupported("asset type".to_string())),
  //     }
  // }

  // pub fn withdraw_msg(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
  //     Ok(CosmosMsg::Wasm(WasmMsg::Execute {
  //         contract_addr: self.0.to_string(),
  //         msg: to_json_binary(&ExecuteMsg::Unstake(asset))?,
  //         funds: vec![],
  //     }))
  // }

  // pub fn set_reward_distribution_msg(
  //     &self,
  //     distribution: Vec<AssetDistribution>,
  // ) -> Result<CosmosMsg, SharedError> {
  //     Ok(CosmosMsg::Wasm(WasmMsg::Execute {
  //         contract_addr: self.0.to_string(),
  //         msg: to_json_binary(&ExecuteMsg::SetAssetRewardDistribution(distribution))?,
  //         funds: vec![],
  //     }))
  // }

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
}
