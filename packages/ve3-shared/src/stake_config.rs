use crate::adapters::asset_staking::AssetStaking;
use crate::adapters::astroport::AstroportIncentives;
use crate::error::SharedError;
use crate::extensions::asset_infos_ext::AssetInfosEx;
use crate::extensions::env_ext::EnvExt;
use crate::msgs_asset_staking::CallbackMsg;
use crate::msgs_asset_staking::ExecuteMsg;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cosmwasm_std::Api;
use cosmwasm_std::CosmosMsg;
use cosmwasm_std::DepsMut;
use cosmwasm_std::Env;
use cw_address_like::AddressLike;
use cw_asset::Asset;
use cw_asset::AssetError;
use cw_asset::AssetInfo;
use cw_asset::AssetInfoBase;

#[cw_serde]
#[derive(Default)]
pub enum StakeConfig<T: AddressLike> {
  #[default]
  Default,
  Astroport {
    contract: T,
    reward_infos: Vec<AssetInfoBase<T>>,
  },
  Ve3 {
    contract: T,
    reward_infos: Vec<AssetInfoBase<T>>,
  },
}

fn track_bribes_callback_msg(
  deps: &DepsMut,
  env: &Env,
  asset: AssetInfo,
  asset_infos: &Vec<AssetInfo>,
) -> Result<CosmosMsg, SharedError> {
  Ok(env.callback_msg(ExecuteMsg::Callback(CallbackMsg::TrackBribes {
    for_asset: asset,
    initial_balances: asset_infos.with_balance_query(&deps.querier, &env.contract.address)?,
  }))?)
}

impl StakeConfig<String> {
  pub fn check(self, api: &dyn Api) -> Result<StakeConfig<Addr>, AssetError> {
    Ok(match self {
      StakeConfig::Default => StakeConfig::Default,
      StakeConfig::Astroport {
        contract,
        reward_infos,
      } => StakeConfig::Astroport {
        contract: api.addr_validate(&contract)?,
        reward_infos: reward_infos
          .into_iter()
          .map(|a| a.check(api, None))
          .collect::<Result<Vec<_>, AssetError>>()?,
      },
      StakeConfig::Ve3 {
        contract,
        reward_infos,
      } => StakeConfig::Ve3 {
        contract: api.addr_validate(&contract)?,
        reward_infos: reward_infos
          .into_iter()
          .map(|a| a.check(api, None))
          .collect::<Result<Vec<_>, AssetError>>()?,
      },
    })
  }
}

impl StakeConfig<Addr> {
  pub fn stake_check_received_msg(
    &self,
    deps: &DepsMut,
    env: &Env,
    asset: Asset,
  ) -> Result<Vec<CosmosMsg>, SharedError> {
    if asset.amount.is_zero() {
      return Ok(vec![]);
    }

    Ok(match self {
      StakeConfig::Default => vec![],
      StakeConfig::Astroport {
        contract,
        reward_infos,
      } => {
        vec![
          AstroportIncentives(contract.clone()).deposit(asset.clone())?,
          track_bribes_callback_msg(deps, env, asset.info, reward_infos)?,
        ]
      },
      StakeConfig::Ve3 {
        contract,
        reward_infos,
      } => {
        vec![
          AssetStaking(contract.clone()).deposit_msg(asset.clone())?,
          track_bribes_callback_msg(deps, env, asset.info, reward_infos)?,
        ]
      },
    })
  }

  pub fn unstake_check_received_msg(
    &self,
    deps: &DepsMut,
    env: &Env,
    asset: Asset,
  ) -> Result<Vec<CosmosMsg>, SharedError> {
    if asset.amount.is_zero() {
      return Ok(vec![]);
    }

    Ok(match self {
      StakeConfig::Default => vec![],
      StakeConfig::Astroport {
        contract,
        reward_infos,
      } => {
        vec![
          AstroportIncentives(contract.clone()).withdraw(asset.clone())?,
          track_bribes_callback_msg(deps, env, asset.info, reward_infos)?,
        ]
      },
      StakeConfig::Ve3 {
        contract,
        reward_infos,
      } => {
        vec![
          AssetStaking(contract.clone()).withdraw_msg(asset.clone())?,
          track_bribes_callback_msg(deps, env, asset.info, reward_infos)?,
        ]
      },
    })
  }

  pub fn claim_check_received_msg(
    &self,
    deps: &DepsMut,
    env: &Env,
    asset: AssetInfo,
  ) -> Result<Vec<CosmosMsg>, SharedError> {
    Ok(match self {
      StakeConfig::Default => vec![],
      StakeConfig::Astroport {
        contract,
        reward_infos,
      } => {
        let asset_string = match &asset {
          cw_asset::AssetInfoBase::Native(native) => Ok(native.to_string()),
          cw_asset::AssetInfoBase::Cw20(contract) => Ok(contract.to_string()),
          _ => Err(SharedError::NotSupported("asset".to_string())),
        }?;

        vec![
          AstroportIncentives(contract.clone()).claim_rewards_msg(vec![asset_string])?,
          track_bribes_callback_msg(deps, env, asset, reward_infos)?,
        ]
      },
      StakeConfig::Ve3 {
        contract,
        reward_infos,
      } => {
        vec![
          AssetStaking(contract.clone()).claim_reward_msg(asset.clone())?,
          track_bribes_callback_msg(deps, env, asset, reward_infos)?,
        ]
      },
    })
  }
}
