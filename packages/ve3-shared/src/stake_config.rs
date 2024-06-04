use crate::adapters::astroport::AstroportIncentives;
use crate::adapters::ve3_asset_staking::Ve3AssetStaking;
use crate::contract_asset_staking::CallbackMsg;
use crate::contract_asset_staking::ExecuteMsg;
use crate::error::SharedError;
use crate::extensions::asset_infos_ext::AssetInfosEx;
use crate::extensions::env_ext::EnvExt;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cosmwasm_std::CosmosMsg;
use cosmwasm_std::DepsMut;
use cosmwasm_std::Env;
use cw_asset::Asset;
use cw_asset::AssetInfo;

#[cw_serde]
#[derive(Default)]
pub enum StakeConfig {
    #[default]
    Default,
    Astroport {
        contract: Addr,
        reward_infos: Vec<AssetInfo>,
    },
    Ve3 {
        contract: Addr,
        reward_infos: Vec<AssetInfo>,
    },
}

fn tributes_callback_msg(
    deps: &DepsMut,
    env: &Env,
    asset: AssetInfo,
    asset_infos: &Vec<AssetInfo>,
) -> Result<CosmosMsg, SharedError> {
    Ok(env.callback_msg(ExecuteMsg::Callback(CallbackMsg::AddTributes {
        asset,
        initial_balances: asset_infos.with_balance_query(&deps.querier, &env.contract.address)?,
    }))?)
}

impl StakeConfig {
    pub fn stake_check_received_msg(
        &self,
        deps: &DepsMut,
        env: &Env,
        asset: Asset,
    ) -> Result<Vec<CosmosMsg>, SharedError> {
        Ok(match self {
            StakeConfig::Default => vec![],
            StakeConfig::Astroport {
                contract,
                reward_infos,
            } => {
                vec![
                    AstroportIncentives(contract.clone()).deposit(asset.clone())?,
                    tributes_callback_msg(deps, env, asset.info, reward_infos)?,
                ]
            },
            StakeConfig::Ve3 {
                contract,
                reward_infos,
            } => {
                vec![
                    Ve3AssetStaking(contract.clone()).deposit(asset.clone())?,
                    tributes_callback_msg(deps, env, asset.info, reward_infos)?,
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
        Ok(match self {
            StakeConfig::Default => vec![],
            StakeConfig::Astroport {
                contract,
                reward_infos,
            } => {
                vec![
                    AstroportIncentives(contract.clone()).withdraw(asset.clone())?,
                    tributes_callback_msg(deps, env, asset.info, reward_infos)?,
                ]
            },
            StakeConfig::Ve3 {
                contract,
                reward_infos,
            } => {
                vec![
                    Ve3AssetStaking(contract.clone()).withdraw(asset.clone())?,
                    tributes_callback_msg(deps, env, asset.info, reward_infos)?,
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
                    tributes_callback_msg(deps, env, asset, reward_infos)?,
                ]
            },
            StakeConfig::Ve3 {
                contract,
                reward_infos,
            } => {
                vec![
                    Ve3AssetStaking(contract.clone()).claim_rewards_msg(vec![asset.clone()])?,
                    tributes_callback_msg(deps, env, asset, reward_infos)?,
                ]
            },
        })
    }
}
