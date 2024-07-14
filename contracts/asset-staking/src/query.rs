#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order, StdResult, Uint128};
use std::cmp::min;
use ve3_shared::{
  constants::SECONDS_PER_YEAR,
  extensions::asset_info_ext::AssetInfoExt,
  helpers::take::compute_balance_amount,
  msgs_asset_staking::{
    AllPendingRewardsQuery, AllStakedBalancesQuery, AssetInfoWithRuntime, AssetQuery,
    PendingRewardsDetailRes, PendingRewardsRes, QueryMsg, StakedBalanceRes,
    WhitelistedAssetsDetailsResponse, WhitelistedAssetsResponse,
  },
};

use crate::state::{
  ASSET_CONFIG, ASSET_REWARD_DISTRIBUTION, ASSET_REWARD_RATE, CONFIG, SHARES, TOTAL,
  UNCLAIMED_REWARDS, USER_ASSET_REWARD_RATE, WHITELIST,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
  Ok(match msg {
    QueryMsg::Config {} => get_config(deps)?,
    QueryMsg::WhitelistedAssets {} => get_whitelisted_assets(deps)?,
    QueryMsg::WhitelistedAssetDetails {} => get_whitelisted_assets_details(deps)?,
    QueryMsg::RewardDistribution {} => get_rewards_distribution(deps)?,
    QueryMsg::StakedBalance(asset_query) => get_staked_balance(deps, env, asset_query)?,
    QueryMsg::PendingRewards(asset_query) => get_pending_rewards(deps, asset_query)?,
    QueryMsg::AllStakedBalances(query) => get_all_staked_balances(deps, env, query)?,
    QueryMsg::AllPendingRewards(query) => get_all_pending_rewards(deps, query)?,
    QueryMsg::AllPendingRewardsDetail(query) => get_all_pending_rewards_detail(deps, env, query)?,
    QueryMsg::TotalStakedBalances {} => get_total_staked_balances(deps, env)?,
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

fn get_whitelisted_assets_details(deps: Deps) -> StdResult<Binary> {
  let whitelist =
    WHITELIST.keys(deps.storage, None, None, Order::Ascending).collect::<StdResult<Vec<_>>>()?;

  let res: WhitelistedAssetsDetailsResponse = ASSET_CONFIG
    .range(deps.storage, None, None, Order::Ascending)
    .map(|a| {
      let (info, config) = a?;

      Ok(AssetInfoWithRuntime {
        whitelisted: whitelist.contains(&info),
        info,
        config,
      })
    })
    .collect::<StdResult<Vec<_>>>()?;

  to_json_binary(&res)
}

fn get_rewards_distribution(deps: Deps) -> StdResult<Binary> {
  let asset_rewards_distr = ASSET_REWARD_DISTRIBUTION.load(deps.storage)?;

  to_json_binary(&asset_rewards_distr)
}

fn get_staked_balance(deps: Deps, env: Env, asset_query: AssetQuery) -> StdResult<Binary> {
  let addr = deps.api.addr_validate(&asset_query.address)?;
  let key = (addr, &asset_query.asset);
  let user_shares = SHARES.load(deps.storage, key)?;
  let (balance, shares) = TOTAL.load(deps.storage, &asset_query.asset)?;
  let mut config = ASSET_CONFIG.may_load(deps.storage, &asset_query.asset)?.unwrap_or_default();

  if config.last_taken_s != 0 {
    let take_diff_s = Uint128::new((env.block.time.seconds() - config.last_taken_s).into());
    let relevant_balance = balance.saturating_sub(config.taken);
    let take_amount = config.yearly_take_rate
      * relevant_balance
        .multiply_ratio(min(take_diff_s, Uint128::new(SECONDS_PER_YEAR.into())), SECONDS_PER_YEAR);
    config.last_taken_s = env.block.time.seconds();
    config.taken = config.taken.checked_add(take_amount)?
  };

  let real_balance =
    compute_balance_amount(shares, user_shares, balance.saturating_sub(config.taken));
  let asset = asset_query.asset.with_balance(real_balance);

  to_json_binary(&StakedBalanceRes {
    asset,
    shares: user_shares,
    total_shares: shares,
    config,
  })
}

fn get_all_staked_balances(
  deps: Deps,
  env: Env,
  asset_query: AllStakedBalancesQuery,
) -> StdResult<Binary> {
  let addr = deps.api.addr_validate(&asset_query.address)?;
  let mut res: Vec<StakedBalanceRes> = Vec::new();

  for shares in SHARES.prefix(addr.clone()).range(deps.storage, None, None, Order::Ascending) {
    let (asset_info, user_shares) = shares?;

    let mut asset_config = ASSET_CONFIG.load(deps.storage, &asset_info)?;

    let (balance, shares) = TOTAL.load(deps.storage, &asset_info).unwrap_or_default();

    if asset_config.last_taken_s != 0 {
      let take_diff_s = Uint128::new((env.block.time.seconds() - asset_config.last_taken_s).into());
      let relevant_balance = balance.saturating_sub(asset_config.taken);
      let take_amount = asset_config.yearly_take_rate
        * relevant_balance.multiply_ratio(
          min(take_diff_s, Uint128::new(SECONDS_PER_YEAR.into())),
          SECONDS_PER_YEAR,
        );
      asset_config.last_taken_s = env.block.time.seconds();
      asset_config.taken = asset_config.taken.checked_add(take_amount)?
    };

    let available = balance.saturating_sub(asset_config.taken);
    let real_balance = compute_balance_amount(shares, user_shares, available);
    let asset = asset_info.with_balance(real_balance);

    // Append the request
    res.push(StakedBalanceRes {
      asset,
      shares: user_shares,
      total_shares: shares,
      config: asset_config,
    })
  }

  to_json_binary(&res)
}

fn get_pending_rewards(deps: Deps, asset_query: AssetQuery) -> StdResult<Binary> {
  let config = CONFIG.load(deps.storage)?;
  let asset_info = asset_query.asset;
  let addr = deps.api.addr_validate(&asset_query.address)?;
  let key = (addr, &asset_info);
  let user_reward_rate = USER_ASSET_REWARD_RATE.load(deps.storage, key.clone())?;
  let asset_reward_rate = ASSET_REWARD_RATE.load(deps.storage, &asset_info)?;
  let user_share = SHARES.load(deps.storage, key.clone())?;
  let unclaimed_rewards = UNCLAIMED_REWARDS.load(deps.storage, key).unwrap_or_default();
  let pending_rewards = (asset_reward_rate - user_reward_rate) * user_share;

  to_json_binary(&PendingRewardsRes {
    staked_asset_share: asset_info.with_balance(user_share),
    reward_asset: config.reward_info.with_balance(unclaimed_rewards + pending_rewards),
  })
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
      let user_share = SHARES.load(deps.storage, (addr.clone(), &asset)).unwrap_or_default();
      let unclaimed_rewards =
        UNCLAIMED_REWARDS.load(deps.storage, (addr.clone(), &asset)).unwrap_or_default();
      let pending_rewards = (asset_reward_rate - user_reward_rate) * user_share;
      Ok(PendingRewardsRes {
        staked_asset_share: asset.with_balance(user_share),
        reward_asset: config.reward_info.with_balance(pending_rewards + unclaimed_rewards),
      })
    })
    .filter(|a| match a {
      Ok(o) => !o.reward_asset.amount.is_zero(),
      Err(_) => true,
    })
    .collect::<StdResult<Vec<PendingRewardsRes>>>();

  to_json_binary(&all_pending_rewards?)
}

fn get_all_pending_rewards_detail(
  deps: Deps,
  env: Env,
  query: AllPendingRewardsQuery,
) -> StdResult<Binary> {
  let config = CONFIG.load(deps.storage)?;
  let addr = deps.api.addr_validate(&query.address)?;
  let all_pending_rewards: StdResult<Vec<PendingRewardsDetailRes>> = USER_ASSET_REWARD_RATE
    .prefix(addr.clone())
    .range(deps.storage, None, None, Order::Ascending)
    .map(|item| {
      let (asset_info, user_reward_rate) = item?;
      let asset_reward_rate = ASSET_REWARD_RATE.load(deps.storage, &asset_info)?;
      let user_shares = SHARES.load(deps.storage, (addr.clone(), &asset_info)).unwrap_or_default();
      let unclaimed_rewards =
        UNCLAIMED_REWARDS.load(deps.storage, (addr.clone(), &asset_info)).unwrap_or_default();
      let pending_rewards = (asset_reward_rate - user_reward_rate) * user_shares;

      let (balance, shares) = TOTAL.load(deps.storage, &asset_info).unwrap_or_default();
      let mut asset_config = ASSET_CONFIG.load(deps.storage, &asset_info)?;

      if asset_config.last_taken_s != 0 {
        let take_diff_s =
          Uint128::new((env.block.time.seconds() - asset_config.last_taken_s).into());
        let relevant_balance = balance.saturating_sub(asset_config.taken);
        let take_amount = asset_config.yearly_take_rate
          * relevant_balance.multiply_ratio(
            min(take_diff_s, Uint128::new(SECONDS_PER_YEAR.into())),
            SECONDS_PER_YEAR,
          );
        asset_config.last_taken_s = env.block.time.seconds();
        asset_config.taken = asset_config.taken.checked_add(take_amount)?
      };

      let real_balance =
        compute_balance_amount(shares, user_shares, balance.saturating_sub(asset_config.taken));

      Ok(PendingRewardsDetailRes {
        share: user_shares,
        staked_asset: asset_info.with_balance(real_balance),
        reward_asset: config.reward_info.with_balance(pending_rewards + unclaimed_rewards),
      })
    })
    .filter(|a| match a {
      Ok(o) => !o.reward_asset.amount.is_zero() || !o.share.is_zero(),
      Err(_) => true,
    })
    .collect::<StdResult<Vec<PendingRewardsDetailRes>>>();

  to_json_binary(&all_pending_rewards?)
}

fn get_total_staked_balances(deps: Deps, env: Env) -> StdResult<Binary> {
  let total_staked_balances: StdResult<Vec<StakedBalanceRes>> = TOTAL
    .range(deps.storage, None, None, Order::Ascending)
    .map(|total_balance| -> StdResult<StakedBalanceRes> {
      let (asset, (balance, shares)) = total_balance?;

      let mut config = ASSET_CONFIG.load(deps.storage, &asset)?;

      if config.last_taken_s != 0 {
        let take_diff_s = Uint128::new((env.block.time.seconds() - config.last_taken_s).into());
        let relevant_balance = balance.saturating_sub(config.taken);
        let take_amount = config.yearly_take_rate
          * relevant_balance.multiply_ratio(
            min(take_diff_s, Uint128::new(SECONDS_PER_YEAR.into())),
            SECONDS_PER_YEAR,
          );
        config.last_taken_s = env.block.time.seconds();
        config.taken = config.taken.checked_add(take_amount)?
      };

      let real_balance = balance.saturating_sub(config.taken);
      let asset = asset.with_balance(real_balance);
      Ok(StakedBalanceRes {
        asset,
        shares,
        total_shares: shares,
        config,
      })
    })
    .collect();
  to_json_binary(&total_staked_balances?)
}
