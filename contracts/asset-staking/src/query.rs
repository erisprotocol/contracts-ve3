#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order, StdResult, Uint128};
use cw_asset::AssetInfo;
use ve3_shared::msgs_asset_staking::{
  AllPendingRewardsQuery, AllStakedBalancesQuery, AssetQuery, PendingRewardsRes, QueryMsg,
  StakedBalanceRes, WhitelistedAssetsResponse,
};

use crate::{
  contract::compute_balance_amount,
  state::{
    ASSET_CONFIG, ASSET_REWARD_DISTRIBUTION, ASSET_REWARD_RATE, BALANCES, CONFIG, TOTAL_BALANCES,
    UNCLAIMED_REWARDS, USER_ASSET_REWARD_RATE, WHITELIST,
  },
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
  Ok(match msg {
    QueryMsg::Config {} => get_config(deps)?,
    QueryMsg::WhitelistedAssets {} => get_whitelisted_assets(deps)?,
    QueryMsg::RewardDistribution {} => get_rewards_distribution(deps)?,
    QueryMsg::StakedBalance(asset_query) => get_staked_balance(deps, asset_query)?,
    QueryMsg::PendingRewards(asset_query) => get_pending_rewards(deps, asset_query)?,
    QueryMsg::AllStakedBalances(query) => get_all_staked_balances(deps, query)?,
    QueryMsg::AllPendingRewards(query) => get_all_pending_rewards(deps, query)?,
    QueryMsg::TotalStakedBalances {} => get_total_staked_balances(deps)?,
  })
}

fn get_config(deps: Deps) -> StdResult<Binary> {
  let cfg = CONFIG.load(deps.storage)?;

  to_json_binary(&cfg)
}

fn get_whitelisted_assets(deps: Deps) -> StdResult<Binary> {
  let whitelist =
    WHITELIST.keys(deps.storage, None, None, Order::Ascending).collect::<StdResult<Vec<_>>>()?;

  let res: WhitelistedAssetsResponse = whitelist;
  to_json_binary(&res)
}

fn get_rewards_distribution(deps: Deps) -> StdResult<Binary> {
  let asset_rewards_distr = ASSET_REWARD_DISTRIBUTION.load(deps.storage)?;

  to_json_binary(&asset_rewards_distr)
}

fn get_staked_balance(deps: Deps, asset_query: AssetQuery) -> StdResult<Binary> {
  let addr = deps.api.addr_validate(&asset_query.address)?;
  let key = (addr, &asset_query.asset);
  let user_shares = BALANCES.load(deps.storage, key)?;
  let (balance, shares) = TOTAL_BALANCES.load(deps.storage, &asset_query.asset)?;
  let config = ASSET_CONFIG.may_load(deps.storage, &asset_query.asset)?.unwrap_or_default();

  to_json_binary(&StakedBalanceRes {
    asset: asset_query.asset,
    balance: compute_balance_amount(shares, user_shares, balance - config.taken),
    shares: user_shares,
    config,
  })
}

fn get_pending_rewards(deps: Deps, asset_query: AssetQuery) -> StdResult<Binary> {
  let config = CONFIG.load(deps.storage)?;
  let asset_info = asset_query.asset;
  let addr = deps.api.addr_validate(&asset_query.address)?;
  let key = (addr, &asset_info);
  let user_reward_rate = USER_ASSET_REWARD_RATE.load(deps.storage, key.clone())?;
  let asset_reward_rate = ASSET_REWARD_RATE.load(deps.storage, &asset_info)?;
  let user_balance = BALANCES.load(deps.storage, key.clone())?;
  let unclaimed_rewards = UNCLAIMED_REWARDS.load(deps.storage, key).unwrap_or(Uint128::zero());
  let pending_rewards = (asset_reward_rate - user_reward_rate) * user_balance;

  to_json_binary(&PendingRewardsRes {
    rewards: unclaimed_rewards + pending_rewards,
    staked_asset: asset_info,
    reward_asset: config.reward_info,
  })
}

fn get_all_staked_balances(deps: Deps, asset_query: AllStakedBalancesQuery) -> StdResult<Binary> {
  let addr = deps.api.addr_validate(&asset_query.address)?;
  let whitelist = WHITELIST.range(deps.storage, None, None, Order::Ascending);
  let mut res: Vec<StakedBalanceRes> = Vec::new();

  for asset_res in whitelist {
    // Build the required key to recover the BALANCES
    let (asset_info, _) = asset_res?;
    let stake_key = (addr.clone(), &asset_info);
    let user_shares = BALANCES.load(deps.storage, stake_key).unwrap_or(Uint128::zero());
    let (balance, shares) = TOTAL_BALANCES.load(deps.storage, &asset_info)?;
    let config = ASSET_CONFIG.load(deps.storage, &asset_info)?;

    // Append the request
    res.push(StakedBalanceRes {
      asset: asset_info,
      balance: compute_balance_amount(shares, user_shares, balance - config.taken),
      shares: user_shares,
      config,
    })
  }

  to_json_binary(&res)
}

fn get_all_pending_rewards(deps: Deps, query: AllPendingRewardsQuery) -> StdResult<Binary> {
  let config = CONFIG.load(deps.storage)?;
  let addr = deps.api.addr_validate(&query.address)?;
  let all_pending_rewards: StdResult<Vec<PendingRewardsRes>> = USER_ASSET_REWARD_RATE
    .prefix(addr.clone())
    .range(deps.storage, None, None, Order::Ascending)
    .map(|item| {
      let (asset, user_reward_rate) = item?;
      let asset_reward_rate = ASSET_REWARD_RATE.load(deps.storage, &asset)?;
      let user_balance = BALANCES.load(deps.storage, (addr.clone(), &asset))?;
      let unclaimed_rewards =
        UNCLAIMED_REWARDS.load(deps.storage, (addr.clone(), &asset)).unwrap_or(Uint128::zero());
      let pending_rewards = (asset_reward_rate - user_reward_rate) * user_balance;
      Ok(PendingRewardsRes {
        rewards: pending_rewards + unclaimed_rewards,
        staked_asset: asset,
        reward_asset: AssetInfo::Native(config.reward_info.to_string()),
      })
    })
    .collect::<StdResult<Vec<PendingRewardsRes>>>();

  to_json_binary(&all_pending_rewards?)
}

fn get_total_staked_balances(deps: Deps) -> StdResult<Binary> {
  let total_staked_balances: StdResult<Vec<StakedBalanceRes>> = TOTAL_BALANCES
    .range(deps.storage, None, None, Order::Ascending)
    .map(|total_balance| -> StdResult<StakedBalanceRes> {
      let (asset, (balance, shares)) = total_balance?;

      let config = ASSET_CONFIG.load(deps.storage, &asset)?;

      Ok(StakedBalanceRes {
        asset,
        balance,
        shares,
        config,
      })
    })
    .collect();
  to_json_binary(&total_staked_balances?)
}
