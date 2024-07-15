use crate::error::ContractError;
use crate::state::{
  fetch_first_gauge_vote, fetch_last_gauge_vote, user_idx, AssetIndex, GaugeDistributionPeriod,
  CONFIG, GAUGE_DISTRIBUTION, REBASE, UNCLAIMED_REBASE, USER_ASSET_REWARD_INDEX,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult, Uint128, Uint256};
use cw_asset::AssetInfoUnchecked;
use cw_storage_plus::Bound;
use std::str::FromStr;
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT};
use ve3_shared::helpers::governance::get_period;
use ve3_shared::helpers::time::{GetPeriod, GetPeriods, Time, Times};
use ve3_shared::msgs_asset_gauge::{
  GaugeDistributionResponse, GaugeInfosResponse, GaugeVote, LastDistributionPeriodResponse,
  QueryMsg, UserFirstParticipationResponse, UserInfoExtendedResponse, UserInfosResponse,
  UserPendingRebaseResponse, UserShare, UserSharesResponse, VotedInfoResponse,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),

    QueryMsg::UserShares {
      user,
      times,
    } => Ok(to_json_binary(&user_shares(deps, env, user, times)?)?),

    QueryMsg::UserFirstParticipation {
      user,
    } => Ok(to_json_binary(&user_first_participation(deps, user)?)?),

    QueryMsg::UserPendingRebase {
      user,
    } => Ok(to_json_binary(&user_pending_rebase(deps, env, user)?)?),

    QueryMsg::UserInfo {
      user,
      time,
    } => Ok(to_json_binary(&user_info(deps, env, user, time)?)?),

    QueryMsg::UserInfos {
      start_after,
      limit,
      time,
    } => Ok(to_json_binary(&user_infos(deps, env, start_after, limit, time)?)?),

    QueryMsg::GaugeInfo {
      time,
      gauge,
      key,
    } => Ok(to_json_binary(&gauge_info(deps, env, gauge, key, time)?)?),

    QueryMsg::GaugeInfos {
      time,
      gauge,
      keys,
    } => Ok(to_json_binary(&gauge_infos(deps, env, gauge, keys, time)?)?),

    QueryMsg::Distribution {
      gauge,
      time,
    } => Ok(to_json_binary(&distribution(deps, env, gauge, time)?)?),

    QueryMsg::Distributions {
      time,
    } => Ok(to_json_binary(&distributions(deps, env, time)?)?),

    QueryMsg::LastDistributions {} => Ok(to_json_binary(&last_distributions(deps, env)?)?),

    QueryMsg::LastDistributionPeriod {} => {
      Ok(to_json_binary(&last_distribution_period(deps, env)?)?)
    },
  }
}

fn user_first_participation(
  deps: Deps,
  user: Addr,
) -> Result<UserFirstParticipationResponse, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let mut first_period = None;

  for gauge_config in config.gauges {
    let gauge = &gauge_config.name;

    let result = fetch_first_gauge_vote(deps.storage, gauge, user.as_str())?;
    if let Some((period, _)) = result {
      first_period = match first_period {
        Some(first_period) => Some(std::cmp::min(first_period, period)),
        None => Some(period),
      }
    }
  }

  Ok(UserFirstParticipationResponse {
    period: first_period,
  })
}

fn user_pending_rebase(
  deps: Deps,
  env: Env,
  user: Addr,
) -> Result<UserPendingRebaseResponse, ContractError> {
  let rebase = REBASE.load(deps.storage)?;
  let block_period = get_period(env.block.time.seconds())?;
  let user_data = user_idx().get_latest_data(deps.storage, block_period + 1, user.as_str())?;

  let balance = user_data.fixed_amount;
  let user_reward_index = USER_ASSET_REWARD_INDEX.load(deps.storage, user.clone());
  let global_reward_index = rebase.global_reward_index;

  if let Ok(user_reward_rate) = user_reward_index {
    let user_staked = balance;
    let user_amount = Uint256::from(user_staked);
    let rewards: Uint128 = ((global_reward_index - user_reward_rate) * user_amount).try_into()?;

    let unclaimed = UNCLAIMED_REBASE.may_load(deps.storage, user)?.unwrap_or_default();

    Ok(UserPendingRebaseResponse {
      rebase: unclaimed + rewards,
    })
  } else {
    Ok(UserPendingRebaseResponse {
      rebase: Uint128::zero(),
    })
  }
}

fn user_shares(
  deps: Deps,
  env: Env,
  user: Addr,
  times: Option<Times>,
) -> Result<UserSharesResponse, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let block_period = get_period(env.block.time.seconds())?;

  let mut response = UserSharesResponse {
    shares: vec![],
  };

  for period in times.get_periods(&env)? {
    if period > block_period {
      return Err(ContractError::PeriodNotFinished(period));
    }

    let user_data = user_idx().get_latest_data(deps.storage, period, user.as_str())?;
    let user_vp = user_data.total_vp()?;
    if user_vp.is_zero() {
      return Err(ContractError::ZeroVotingPower(user.to_string(), period));
    }

    for gauge_config in config.gauges.iter() {
      let gauge = &gauge_config.name;

      let distribution = GAUGE_DISTRIBUTION
        .load(deps.storage, (gauge, period))
        .map_err(|_| ContractError::GaugeDistributionNotExecuted(gauge.to_string(), period))?;
      let user_vote = fetch_last_gauge_vote(deps.storage, gauge, user.as_str(), period)?;

      if let Some((_, votes)) = user_vote {
        for (asset, bps) in votes.votes {
          if bps.is_zero() {
            continue;
          }

          let asset = AssetInfoUnchecked::from_str(&asset)?.check(deps.api, None)?;

          let vp = bps * user_vp;
          let total_vp = distribution
            .assets
            .iter()
            .find(|a| a.asset == asset)
            .map(|a| a.total_vp)
            .unwrap_or_default();

          let share = UserShare {
            gauge: gauge.to_string(),
            asset,
            period,
            user_vp: vp,
            total_vp,
          };

          response.shares.push(share);
        }
      }
    }
  }

  Ok(response)
}

/// Returns user information.
fn user_info(
  deps: Deps,
  env: Env,
  user: String,
  time: Option<Time>,
) -> StdResult<UserInfoExtendedResponse> {
  deps.api.addr_validate(&user)?;

  let period = time.get_period(&env)?;

  let info = user_idx().get_latest_data(deps.storage, period, &user)?;

  let gauges = CONFIG.load(deps.storage)?.gauges;
  let mut gauge_votes = vec![];
  for gauge in gauges {
    if let Some((period, votes)) = fetch_last_gauge_vote(deps.storage, &gauge.name, &user, period)?
    {
      gauge_votes.push(GaugeVote {
        gauge: gauge.name,
        period,
        votes: votes.votes.into_iter().map(|(a, b)| (a, b.u16())).collect(),
      })
    }
  }

  Ok(UserInfoExtendedResponse {
    voting_power: info.voting_power,
    fixed_amount: info.fixed_amount,
    slope: info.slope,
    gauge_votes,
  })
}

// returns all user votes
fn user_infos(
  deps: Deps,
  env: Env,
  start_after: Option<String>,
  limit: Option<u32>,
  time: Option<Time>,
) -> StdResult<UserInfosResponse> {
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
  let period = time.get_period(&env)?;
  let idx = user_idx();

  let mut start: Option<Bound<&str>> = None;
  let addr: Addr;
  if let Some(start_after) = start_after {
    if let Ok(start_after_addr) = deps.api.addr_validate(&start_after) {
      addr = start_after_addr;
      start = Some(Bound::exclusive(addr.as_str()));
    }
  }

  let mut result = vec![];

  for key in idx.keys.range(deps.storage, start, None, cosmwasm_std::Order::Ascending).take(limit) {
    let (key, _) = key?;
    let data = idx.get_latest_data(deps.storage, period, &key)?;
    result.push((
      Addr::unchecked(key),
      VotedInfoResponse {
        voting_power: data.voting_power,
        fixed_amount: data.fixed_amount,
        slope: data.slope,
      },
    ))
  }

  Ok(result)
}

fn distribution(
  deps: Deps,
  env: Env,
  gauge: String,
  time: Option<Time>,
) -> StdResult<GaugeDistributionResponse> {
  let period = time.get_period(&env)?;
  get_gauge_distribution(deps, gauge, period)
}

fn get_gauge_distribution(
  deps: Deps,
  gauge: String,
  period: u64,
) -> StdResult<GaugeDistributionResponse> {
  GAUGE_DISTRIBUTION.may_load(deps.storage, (&gauge, period)).map(|distribution| {
    let distribution = distribution.unwrap_or_default();
    GaugeDistributionResponse {
      gauge,
      period,
      total_gauge_vp: distribution.total_gauge_vp,
      assets: distribution.assets,
    }
  })
}

fn get_last_gauge_distribution(deps: Deps, gauge: String) -> StdResult<GaugeDistributionResponse> {
  let elements = GAUGE_DISTRIBUTION
    .prefix(&gauge)
    .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
    .take(1)
    .collect::<StdResult<Vec<_>>>()?;

  let (period, distribution) =
    elements.first().unwrap_or(&(0, GaugeDistributionPeriod::default())).clone();

  Ok(GaugeDistributionResponse {
    gauge,
    period,
    total_gauge_vp: distribution.total_gauge_vp,
    assets: distribution.assets,
  })
}

fn distributions(
  deps: Deps,
  env: Env,
  time: Option<Time>,
) -> StdResult<Vec<GaugeDistributionResponse>> {
  let config = CONFIG.load(deps.storage)?;
  let period = time.get_period(&env)?;

  config
    .gauges
    .into_iter()
    .map(|a| get_gauge_distribution(deps, a.name, period))
    .collect::<StdResult<Vec<_>>>()
}

fn last_distributions(deps: Deps, _env: Env) -> StdResult<Vec<GaugeDistributionResponse>> {
  let config = CONFIG.load(deps.storage)?;

  config
    .gauges
    .into_iter()
    .map(|a| get_last_gauge_distribution(deps, a.name))
    .collect::<StdResult<Vec<_>>>()
}

fn last_distribution_period(deps: Deps, _env: Env) -> StdResult<LastDistributionPeriodResponse> {
  let config = CONFIG.load(deps.storage)?;

  for gauge in config.gauges {
    let elements = GAUGE_DISTRIBUTION
      .prefix(&gauge.name)
      .keys(deps.storage, None, None, cosmwasm_std::Order::Descending)
      .take(1)
      .collect::<StdResult<Vec<_>>>()?;

    if !elements.is_empty() {
      return Ok(LastDistributionPeriodResponse {
        period: elements.first().cloned(),
      });
    }
  }

  Ok(LastDistributionPeriodResponse {
    period: None,
  })
}

fn gauge_info(
  deps: Deps,
  env: Env,
  gauge: String,
  key: String,
  time: Option<Time>,
) -> StdResult<VotedInfoResponse> {
  let period = time.get_period(&env)?;
  let idx = AssetIndex::new(&gauge);
  let idx = idx.idx();
  let info = idx.get_latest_data(deps.storage, period, &key)?;

  Ok(VotedInfoResponse {
    voting_power: info.voting_power,
    fixed_amount: info.fixed_amount,
    slope: info.slope,
  })
}

fn gauge_infos(
  deps: Deps,
  env: Env,
  gauge: String,
  keys: Option<Vec<String>>,
  time: Option<Time>,
) -> StdResult<GaugeInfosResponse> {
  let period = time.get_period(&env)?;
  let idx = AssetIndex::new(&gauge);
  let idx = idx.idx();

  let keys = if let Some(keys) = keys {
    keys
  } else {
    idx
      .keys
      .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
      .map(|a| Ok(a?.0))
      .collect::<StdResult<Vec<_>>>()?
  };

  let mut result = vec![];
  for key in keys {
    let data = idx.get_latest_data(deps.storage, period, &key)?;
    result.push((
      key,
      VotedInfoResponse {
        voting_power: data.voting_power,
        fixed_amount: data.fixed_amount,
        slope: data.slope,
      },
    ))
  }

  Ok(result)
}
