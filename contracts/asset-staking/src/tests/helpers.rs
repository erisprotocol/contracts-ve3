use crate::contract::{execute, instantiate};
use crate::query::query;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{
  coin, from_json, to_json_binary, Decimal, Deps, DepsMut, Empty, OwnedDeps, Response, Uint128,
};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoBase};
use ve3_shared::msgs_asset_staking::*;

use super::custom_querier::CustomQuerier;

pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier, Empty> {
  OwnedDeps {
    storage: MockStorage::default(),
    api: MockApi::default(),
    querier: CustomQuerier::default(),
    custom_query_type: std::marker::PhantomData,
  }
}

pub fn setup_contract(deps: DepsMut) -> Response {
  let info = mock_info("admin", &[]);
  let env = mock_env();

  let init_msg = InstantiateMsg {
    default_yearly_take_rate: Decimal::percent(10),
    gauge: "stable".to_string(),
    global_config_addr: "global_config".to_string(),
    reward_info: AssetInfoBase::Native("uluna".to_string()),
  };
  instantiate(deps, env, info, init_msg).unwrap()
}

#[track_caller]
pub fn whitelist_assets(deps: DepsMut, assets: Vec<AssetInfoWithConfig<String>>) -> Response {
  let info = mock_info("gov", &[]);
  let env = mock_env();

  let msg = ExecuteMsg::WhitelistAssets(assets);
  execute(deps, env, info, msg).unwrap()
}

#[track_caller]
pub fn remove_assets(deps: DepsMut, assets: Vec<AssetInfo>) -> Response {
  let info = mock_info("gov", &[]);
  let env = mock_env();

  let msg = ExecuteMsg::RemoveAssets(assets);
  execute(deps, env, info, msg).unwrap()
}

#[track_caller]
pub fn stake(deps: DepsMut, user: &str, amount: u128, denom: &str) -> Response {
  let info = mock_info(user, &[coin(amount, denom)]);
  let env = mock_env();
  let msg = ExecuteMsg::Stake {
    recipient: None,
  };
  execute(deps, env, info, msg).unwrap()
}

#[track_caller]
pub fn stake_cw20(deps: DepsMut, user: &str, amount: u128, denom: &str) -> Response {
  let info = mock_info(denom, &[]);
  let env = mock_env();
  let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
    sender: user.to_string(),
    amount: amount.into(),
    msg: to_json_binary(&Cw20HookMsg::Stake {
      recipient: None,
    })
    .unwrap(),
  });
  execute(deps, env, info, msg).unwrap()
}

pub fn unstake(deps: DepsMut, user: &str, amount: u128, denom: &str) -> Response {
  let info = mock_info(user, &[]);
  let env = mock_env();
  let msg = ExecuteMsg::Unstake {
    asset: Asset::native(denom, amount),
    recipient: None,
  };
  execute(deps, env, info, msg).unwrap()
}

pub fn unstake_cw20(deps: DepsMut, user: &str, amount: u128, denom: &str) -> Response {
  let info = mock_info(user, &[]);
  let env = mock_env();
  let msg = ExecuteMsg::Unstake {
    asset: Asset::cw20(deps.api.addr_validate(denom).unwrap(), amount),
    recipient: None,
  };
  execute(deps, env, info, msg).unwrap()
}

pub fn claim_rewards(deps: DepsMut, user: &str, denom: &str) -> Response {
  let info = mock_info(user, &[]);
  let env = mock_env();
  let msg = ExecuteMsg::ClaimReward(AssetInfo::Native(denom.to_string()));
  execute(deps, env, info, msg).unwrap()
}

pub fn query_rewards(deps: Deps, user: &str, denom: &str) -> PendingRewardsRes {
  from_json(
    query(
      deps,
      mock_env(),
      QueryMsg::PendingRewards(AssetQuery {
        address: user.to_string(),
        asset: AssetInfo::Native(denom.to_string()),
      }),
    )
    .unwrap(),
  )
  .unwrap()
}

pub fn query_all_rewards(deps: Deps, user: &str) -> Vec<PendingRewardsRes> {
  from_json(
    query(
      deps,
      mock_env(),
      QueryMsg::AllPendingRewards(AllPendingRewardsQuery {
        address: user.to_string(),
      }),
    )
    .unwrap(),
  )
  .unwrap()
}

pub fn query_all_staked_balances(deps: Deps) -> Vec<StakedBalanceRes> {
  from_json(query(deps, mock_env(), QueryMsg::TotalStakedBalances {}).unwrap()).unwrap()
}

pub fn query_asset_reward_distribution(deps: Deps) -> Vec<AssetDistribution> {
  from_json(query(deps, mock_env(), QueryMsg::RewardDistribution {}).unwrap()).unwrap()
}

#[inline]
pub fn asset_distribution_1() -> Vec<AssetDistribution> {
  vec![
    AssetDistribution {
      asset: AssetInfo::Native("aWHALE".to_string()),
      distribution: Decimal::percent(50),
      total_vp: Uint128::new(100u128),
    },
    AssetDistribution {
      asset: AssetInfo::Native("bWHALE".to_string()),
      distribution: Decimal::percent(50),
      total_vp: Uint128::new(100u128),
    },
  ]
}

#[inline]
pub fn asset_distribution_2() -> Vec<AssetDistribution> {
  vec![
    AssetDistribution {
      asset: AssetInfo::Native("aWHALE".to_string()),
      distribution: Decimal::percent(40),
      total_vp: Uint128::new(100u128),
    },
    AssetDistribution {
      asset: AssetInfo::Native("bWHALE".to_string()),
      distribution: Decimal::percent(40),
      total_vp: Uint128::new(100u128),
    },
    AssetDistribution {
      asset: AssetInfo::Native("willy".to_string()),
      distribution: Decimal::percent(20),
      total_vp: Uint128::new(100u128),
    },
  ]
}

#[inline]
pub fn asset_distribution_broken_1() -> Vec<AssetDistribution> {
  vec![
    AssetDistribution {
      asset: AssetInfo::Native("aWHALE".to_string()),
      distribution: Decimal::percent(40),
      total_vp: Uint128::new(100u128),
    },
    AssetDistribution {
      asset: AssetInfo::Native("bWHALE".to_string()),
      distribution: Decimal::percent(70),
      total_vp: Uint128::new(100u128),
    },
  ]
}

#[inline]
pub fn asset_distribution_broken_2() -> Vec<AssetDistribution> {
  vec![
    AssetDistribution {
      asset: AssetInfo::Native("aWHALE".to_string()),
      distribution: Decimal::percent(40),
      total_vp: Uint128::new(100u128),
    },
    AssetDistribution {
      asset: AssetInfo::Native("bWHALE".to_string()),
      distribution: Decimal::percent(20),
      total_vp: Uint128::new(100u128),
    },
  ]
}
