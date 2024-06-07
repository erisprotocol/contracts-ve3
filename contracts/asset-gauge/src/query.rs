use std::str::FromStr;

use crate::error::ContractError;
use crate::state::{
  fetch_first_gauge_vote, fetch_last_gauge_vote, user_idx, CONFIG, GAUGE_DISTRIBUTION,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env};
use cw_asset::AssetInfoUnchecked;
use ve3_shared::helpers::governance::get_period;
use ve3_shared::helpers::time::{GetPeriods, Times};
use ve3_shared::msgs_asset_gauge::{
  QueryMsg, UserFirstParticipationResponse, UserShare, UserSharesResponse,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    // QueryMsg::UserInfo {
    //     user,
    // } => to_json_binary(&user_info(deps, env, user)?),
    // QueryMsg::UserInfos {
    //     start_after,
    //     limit,
    // } => to_json_binary(&user_infos(deps, env, start_after, limit)?),
    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),

    QueryMsg::UserShares {
      user,
      times,
    } => Ok(to_json_binary(&user_shares(deps, env, user, times)?)?),

    QueryMsg::UserFirstParticipation {
      user,
    } => Ok(to_json_binary(&user_first_participation(deps, user)?)?),
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
            vp,
            total_vp,
          };

          response.shares.push(share);
        }
      }
    }
  }

  Ok(response)
}

// /// Returns user information.
// fn user_info(deps: Deps, env: Env, user: String) -> StdResult<UserInfoResponse> {
//     let user_addr = deps.api.addr_validate(&user)?;
//     let user = USER_INFO
//         .may_load(deps.storage, &user_addr)?
//         .ok_or_else(|| StdError::generic_err("User not found"))?;

//     let block_period = get_period(env.block.time.seconds())?;
//     UserInfo::into_response(user, block_period)
// }

// // returns all user votes
// fn user_infos(
//     deps: Deps,
//     env: Env,
//     start_after: Option<String>,
//     limit: Option<u32>,
// ) -> StdResult<UserInfosResponse> {
//     let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

//     let mut start: Option<Bound<&Addr>> = None;
//     let addr: Addr;
//     if let Some(start_after) = start_after {
//         if let Ok(start_after_addr) = deps.api.addr_validate(&start_after) {
//             addr = start_after_addr;
//             start = Some(Bound::exclusive(&addr));
//         }
//     }

//     let block_period = get_period(env.block.time.seconds())?;

//     let users = USER_INFO
//         .range(deps.storage, start, None, Order::Ascending)
//         .take(limit)
//         .map(|item| {
//             let (user, v) = item?;
//             Ok((user, UserInfo::into_response(v, block_period)?))
//         })
//         .collect::<StdResult<Vec<(Addr, UserInfoResponse)>>>()?;

//     Ok(UserInfosResponse {
//         users,
//     })
// }

// /// Returns all active validators info at a specified period.
// fn validator_infos(
//     deps: Deps,
//     env: Env,
//     validator_addrs: Option<Vec<String>>,
//     period: Option<u64>,
// ) -> StdResult<Vec<(String, VotedValidatorInfoResponse)>> {
//     let period = period.unwrap_or(get_period(env.block.time.seconds())?);

//     // use active validators as fallback
//     let validator_addrs = validator_addrs.unwrap_or_else(|| {
//         let active_validators = VALIDATORS
//             .keys(deps.storage, None, None, Order::Ascending)
//             .collect::<StdResult<Vec<_>>>();

//         active_validators.unwrap_or_default()
//     });

//     let validator_infos: Vec<_> = validator_addrs
//         .into_iter()
//         .map(|validator_addr| {
//             let validator_info = get_asset_info(deps.storage, period, &validator_addr)?;
//             Ok((validator_addr, validator_info))
//         })
//         .collect::<StdResult<Vec<_>>>()?;

//     Ok(validator_infos)
// }

// /// Returns pool's voting information at a specified period.
// fn validator_info(
//     deps: Deps,
//     env: Env,
//     validator_addr: String,
//     period: Option<u64>,
// ) -> StdResult<VotedValidatorInfoResponse> {
//     let block_period = get_period(env.block.time.seconds())?;
//     let period = period.unwrap_or(block_period);
//     get_asset_info(deps.storage, period, &validator_addr)
// }
