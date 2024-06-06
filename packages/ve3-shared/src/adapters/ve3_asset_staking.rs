use crate::{
    contract_asset_staking::{
        AssetDistribution, Cw20HookMsg, ExecuteMsg, QueryMsg, WhitelistedAssetsResponse,
    },
    error::SharedError,
};
use cosmwasm_std::{coins, to_json_binary, Addr, CosmosMsg, QuerierWrapper, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetInfo};

pub struct Ve3AssetStaking(pub Addr);

impl Ve3AssetStaking {
    pub fn claim_rewards_msg(&self, lp_tokens: Vec<AssetInfo>) -> Result<CosmosMsg, SharedError> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&ExecuteMsg::ClaimRewardsMultiple(lp_tokens))?,
            funds: vec![],
        }))
    }

    pub fn deposit_msg(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
        match asset.info {
            cw_asset::AssetInfoBase::Native(native) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_json_binary(&ExecuteMsg::Stake {
                    recipient: None,
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
                        recipient: None,
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
}
