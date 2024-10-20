use crate::{
  error::SharedError,
  extensions::asset_info_ext::AssetInfoExt,
  msgs_asset_staking::{
    AssetConfigRuntime, AssetDistribution, AssetQuery, Cw20HookMsg, ExecuteMsg, QueryMsg,
    StakedBalanceRes, WhitelistedAssetsResponse,
  },
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coins, to_json_binary, Addr, CosmosMsg, QuerierWrapper, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetInfo};

#[cw_serde]
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
        recipient: None,
      })?,
      funds: vec![],
    }))
  }
  pub fn claim_reward_msg(
    &self,
    asset: AssetInfo,
    recipient: Option<String>,
  ) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::ClaimReward {
        asset,
        recipient,
      })?,
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

  pub fn withdraw_msg(
    &self,
    asset: Asset,
    recipient: Option<String>,
  ) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::Unstake {
        asset,
        recipient,
      })?,
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

  pub fn query_staked_balance_fallback(
    &self,
    querier: &QuerierWrapper,
    user: &Addr,
    asset: AssetInfo,
  ) -> Result<StakedBalanceRes, SharedError> {
    let staked: StakedBalanceRes = querier
      .query_wasm_smart(
        self.0.clone(),
        &QueryMsg::StakedBalance(AssetQuery {
          address: user.to_string(),
          asset: asset.clone(),
        }),
      )
      .unwrap_or(StakedBalanceRes {
        asset: asset.with_balance_u128(0),
        shares: Uint128::zero(),
        total_shares: Uint128::zero(),
        config: AssetConfigRuntime::default(),
      });
    Ok(staked)
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
