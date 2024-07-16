use crate::{
  error::SharedError,
  msgs_asset_staking::{
    AssetDistribution, Cw20HookMsg, ExecuteMsg, QueryMsg, WhitelistedAssetsResponse,
  },
};
use cosmwasm_std::{coins, to_json_binary, Addr, CosmosMsg, QuerierWrapper, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetInfo};

pub struct AssetStaking(pub Addr);

impl AssetStaking {
  pub fn claim_rewards_msg(
    &self,
    assets: Option<Vec<AssetInfo>>,
  ) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::ClaimRewards {
        assets,
      })?,
      funds: vec![],
    }))
  }
  pub fn claim_reward_msg(&self, asset: AssetInfo) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::ClaimReward(asset))?,
      funds: vec![],
    }))
  }

  pub fn deposit_msg(
    &self,
    asset: Asset,
    recipient: Option<String>,
  ) -> Result<CosmosMsg, SharedError> {
    match asset.info {
      cw_asset::AssetInfoBase::Native(native) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: self.0.to_string(),
        msg: to_json_binary(&ExecuteMsg::Stake {
          recipient,
        })?,
        funds: coins(asset.amount.u128(), native),
      })),
      cw_asset::AssetInfoBase::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
          contract: self.0.to_string(),
          amount: asset.amount,
          msg: to_json_binary(&Cw20HookMsg::Stake {
            recipient,
          })?,
        })?,
      })),
      _ => Err(SharedError::NotSupported("asset type".to_string())),
    }
  }

  pub fn withdraw_msg(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::Unstake(asset))?,
      funds: vec![],
    }))
  }

  pub fn set_reward_distribution_msg(
    &self,
    distribution: Vec<AssetDistribution>,
  ) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::SetAssetRewardDistribution(distribution))?,
      funds: vec![],
    }))
  }

  pub fn query_whitelisted_assets(
    &self,
    querier: &QuerierWrapper,
  ) -> Result<WhitelistedAssetsResponse, SharedError> {
    let assets: WhitelistedAssetsResponse =
      querier.query_wasm_smart(self.0.clone(), &QueryMsg::WhitelistedAssets {})?;
    Ok(assets)
  }

  pub fn query_whitelisted_assets_str(
    &self,
    querier: &QuerierWrapper,
  ) -> Result<Vec<String>, SharedError> {
    Ok(
      self
        .query_whitelisted_assets(querier)?
        .into_iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>(),
    )
  }
}
