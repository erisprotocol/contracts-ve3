use crate::constants::CONTRACT_TOTAL_VP_TOKEN_ID;
use crate::error::ContractError;
use crate::state::{Point, BLACKLIST, CONFIG, LOCKED};
use crate::utils::{calc_voting_power, fetch_last_checkpoint, fetch_slope_changes};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult, Uint128};
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT, MAX_LOCK_PERIODS};
use ve3_shared::helpers::slope::calc_coefficient;
use ve3_shared::helpers::time::{GetPeriod, Time};
use ve3_shared::msgs_voting_escrow::{
  End, LockInfoResponse, QueryMsg, VeNftCollection, VotingPowerFixedResponse, VotingPowerResponse,
};

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::TotalVotingPower {}** Fetch the total voting power (vAMP supply) at the current block.
///
/// * **QueryMsg::UserVotingPower { user }** Fetch the user's voting power (vAMP balance) at the current block.
///
/// * **QueryMsg::TotalVotingPowerAt { time }** Fetch the total voting power (vAMP supply) at a specified timestamp.
///
/// * **QueryMsg::UserVotingPowerAt { time }** Fetch the user's voting power (vAMP balance) at a specified timestamp.
///
/// * **QueryMsg::LockInfo { user }** Fetch a user's lock information.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  let nft = VeNftCollection::default();
  match msg {
    QueryMsg::BlacklistedVoters {
      start_after,
      limit,
    } => Ok(to_json_binary(&get_blacklisted_voters(deps, start_after, limit)?)?),
    QueryMsg::TotalVamp {
      time,
    } => Ok(to_json_binary(&get_total_vamp_at_time(deps, env, time)?)?),
    QueryMsg::TotalFixed {
      time,
    } => Ok(to_json_binary(&get_total_fixed_at_time(deps, env, time)?)?),
    QueryMsg::LockVamp {
      time,
      token_id,
    } => Ok(to_json_binary(&get_token_vamp_at_time(deps, env, token_id, time)?)?),

    QueryMsg::LockInfo {
      token_id,
      time,
    } => Ok(to_json_binary(&get_token_lock_info(deps, &env, &token_id, time)?)?),

    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),

    _ => Ok(nft.query(deps, env, msg.into())?),
  }
}

/// Returns a list of blacklisted voters.
///
/// * **start_after** is an optional field that specifies whether the function should return
/// a list of voters starting from a specific address onward.
///
/// * **limit** max amount of voters addresses to return.
pub fn get_blacklisted_voters(
  deps: Deps,
  start_after: Option<String>,
  limit: Option<u32>,
) -> Result<Vec<Addr>, ContractError> {
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
  let mut black_list = BLACKLIST.load(deps.storage)?;

  if black_list.is_empty() {
    return Ok(vec![]);
  }

  black_list.sort();

  let mut start_index = Default::default();
  if let Some(start_after) = start_after {
    let start_addr = deps.api.addr_validate(start_after.as_str())?;
    start_index = black_list
      .iter()
      .position(|addr| *addr == start_addr)
      .ok_or_else(|| ContractError::AddressNotBlacklisted(start_addr.to_string()))?
      + 1; // start from the next element of the slice
  }

  // validate end index of the slice
  let end_index = (start_index + limit).min(black_list.len());

  Ok(black_list[start_index..end_index].to_vec())
}

/// Return a user's lock information.
///
/// * **user** user for which we return lock information.
pub fn get_token_lock_info(
  deps: Deps,
  env: &Env,
  token_id: &String,
  time: Option<Time>,
) -> Result<LockInfoResponse, ContractError> {
  if let Some(lock) = LOCKED.may_load(deps.storage, token_id)? {
    let period = time.get_period(env)?;

    let last_checkpoint = fetch_last_checkpoint(deps.storage, token_id, period)?;
    // The voting power point at the specified `time` was found
    let (voting_power, slope, fixed_amount) =
      if let Some(point) = last_checkpoint.map(|(_, point)| point) {
        if point.start == period {
          (point.power, point.slope, point.fixed)
        } else {
          // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
          (calc_voting_power(&point, period), point.slope, point.fixed)
        }
      } else {
        (Uint128::zero(), Uint128::zero(), Uint128::zero())
      };

    let coefficient = if let End::Period(end) = lock.end {
      calc_coefficient(end - lock.last_extend_lock_period)
    } else {
      calc_coefficient(MAX_LOCK_PERIODS)
    };

    let resp = LockInfoResponse {
      from_period: period,

      owner: lock.owner,
      asset: lock.asset,
      underlying_amount: lock.underlying_amount,
      start: lock.start,
      end: lock.end,

      coefficient,
      voting_power,
      fixed_amount,
      slope,
    };
    Ok(resp)
  } else {
    Err(ContractError::LockDoesNotExist(token_id.to_string()))
  }
}

/// Calculates the total voting power (total vAMP supply) at the given period number.
///
/// * **period** period number at which we fetch the total voting power (vAMP supply).
fn get_total_vamp_at_time(
  deps: Deps,
  env: Env,
  time: Option<Time>,
) -> StdResult<VotingPowerResponse> {
  let period = time.get_period(&env)?;
  let last_checkpoint = fetch_last_checkpoint(deps.storage, CONTRACT_TOTAL_VP_TOKEN_ID, period)?;

  let point = last_checkpoint.map_or(
    Point {
      power: Uint128::zero(),
      start: period,
      end: End::Period(period),
      slope: Default::default(),
      fixed: Uint128::zero(),
    },
    |(_, point)| point,
  );
  let fixed = point.fixed;

  let voting_power = if point.start == period {
    point.power + point.fixed
  } else {
    let scheduled_slope_changes = fetch_slope_changes(deps.storage, point.start, period)?;
    let mut init_point = point;
    for (recalc_period, scheduled_change) in scheduled_slope_changes {
      init_point = Point {
        power: calc_voting_power(&init_point, recalc_period),
        start: recalc_period,
        slope: init_point.slope - scheduled_change,
        fixed: init_point.fixed,
        ..init_point
      }
    }
    calc_voting_power(&init_point, period) + init_point.fixed
  };

  Ok(VotingPowerResponse {
    vp: voting_power,
    fixed,
    voting_power: voting_power.saturating_sub(fixed),
  })
}

fn get_total_fixed_at_time(
  deps: Deps,
  env: Env,
  time: Option<Time>,
) -> StdResult<VotingPowerFixedResponse> {
  let period = time.get_period(&env)?;
  let last_checkpoint = fetch_last_checkpoint(deps.storage, CONTRACT_TOTAL_VP_TOKEN_ID, period)?;

  let point = last_checkpoint.map_or(
    Point {
      power: Uint128::zero(),
      start: period,
      end: End::Period(period),
      slope: Default::default(),
      fixed: Uint128::zero(),
    },
    |(_, point)| point,
  );

  Ok(VotingPowerFixedResponse {
    fixed: point.fixed,
  })
}

/// Calculates a user's voting power at a given period number.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
///
/// * **period** period number at which to fetch the user's voting power (vAMP balance).
fn get_token_vamp_at_time(
  deps: Deps,
  env: Env,
  token_id: String,
  time: Option<Time>,
) -> StdResult<VotingPowerResponse> {
  let period = time.get_period(&env)?;
  let last_checkpoint = fetch_last_checkpoint(deps.storage, &token_id, period)?;

  if let Some(point) = last_checkpoint.map(|(_, point)| point) {
    // The voting power point at the specified `time` was found
    let voting_power = if point.start == period {
      point.power + point.fixed
    } else if let End::Period(end) = point.end {
      if end <= period {
        // the current period is after the voting end -> get default end power.
        point.fixed
      } else {
        // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
        calc_voting_power(&point, period) + point.fixed
      }
    } else {
      // without an end, it is full VP
      point.fixed + point.power
    };

    Ok(VotingPowerResponse {
      vp: voting_power,
      fixed: point.fixed,
      voting_power: voting_power.saturating_sub(point.fixed),
    })
  } else {
    // User not found
    Ok(VotingPowerResponse {
      vp: Uint128::zero(),
      fixed: Uint128::zero(),
      voting_power: Uint128::zero(),
    })
  }
}
