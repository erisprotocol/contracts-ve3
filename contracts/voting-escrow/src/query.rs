use crate::error::ContractError;
use crate::state::{Point, BLACKLIST, CONFIG, LOCKED};
use crate::utils::{calc_voting_power, fetch_last_checkpoint, fetch_slope_changes};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult, Uint128};
use cw20::{BalanceResponse, TokenInfoResponse};
use cw20_base::contract::{query_download_logo, query_marketing_info};
use cw20_base::state::TOKEN_INFO;
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT};
use ve3_shared::helpers::governance::get_period;
use ve3_shared::helpers::slope::calc_coefficient;
use ve3_shared::voting_escrow::{
    BlacklistedVotersResponse, Config, LockInfoResponse, QueryMsg, VotingPowerResponse,
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
    match msg {
        QueryMsg::CheckVotersAreBlacklisted {
            voters,
        } => Ok(to_json_binary(&check_voters_are_blacklisted(deps, voters)?)?),
        QueryMsg::BlacklistedVoters {
            start_after,
            limit,
        } => Ok(to_json_binary(&get_blacklisted_voters(deps, start_after, limit)?)?),
        QueryMsg::TotalVamp {} => Ok(to_json_binary(&get_total_vamp(deps, env, None)?)?),
        QueryMsg::UserVamp {
            user,
        } => Ok(to_json_binary(&get_user_vamp(deps, env, user, None)?)?),
        QueryMsg::TotalVampAt {
            time,
        } => Ok(to_json_binary(&get_total_vamp(deps, env, Some(time))?)?),
        QueryMsg::TotalVampAtPeriod {
            period,
        } => Ok(to_json_binary(&get_total_vamp_at_period(deps, env, period)?)?),
        QueryMsg::UserVampAt {
            user,
            time,
        } => Ok(to_json_binary(&get_user_vamp(deps, env, user, Some(time))?)?),
        QueryMsg::UserVampAtPeriod {
            user,
            period,
        } => Ok(to_json_binary(&get_user_vamp_at_period(deps, user, period)?)?),
        QueryMsg::LockInfo {
            user,
        } => Ok(to_json_binary(&get_user_lock_info(deps, &env, user)?)?),
        QueryMsg::UserDepositAtHeight {
            user,
            height,
        } => Ok(to_json_binary(&get_user_deposit_at_height(deps, user, height)?)?),
        QueryMsg::Config {} => {
            let config = CONFIG.load(deps.storage)?;
            Ok(to_json_binary(&config)?)
        },
        QueryMsg::Balance {
            address,
        } => Ok(to_json_binary(&get_user_balance(deps, env, address)?)?),
        QueryMsg::TokenInfo {} => Ok(to_json_binary(&query_token_info(deps, env)?)?),
        QueryMsg::MarketingInfo {} => Ok(to_json_binary(&query_marketing_info(deps)?)?),
        QueryMsg::DownloadLogo {} => Ok(to_json_binary(&query_download_logo(deps)?)?),
    }
}

/// Checks if specified addresses are blacklisted.
///
/// * **voters** addresses to check if they are blacklisted.
pub fn check_voters_are_blacklisted(
    deps: Deps,
    voters: Vec<String>,
) -> Result<BlacklistedVotersResponse, ContractError> {
    let black_list = BLACKLIST.load(deps.storage)?;

    for voter in voters {
        let voter_addr = deps.api.addr_validate(voter.as_str())?;
        if !black_list.contains(&voter_addr) {
            return Ok(BlacklistedVotersResponse::VotersNotBlacklisted {
                voter,
            });
        }
    }

    Ok(BlacklistedVotersResponse::VotersBlacklisted {})
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
fn get_user_lock_info(
    deps: Deps,
    env: &Env,
    user: String,
) -> Result<LockInfoResponse, ContractError> {
    let addr = deps.api.addr_validate(&user)?;
    if let Some(lock) = LOCKED.may_load(deps.storage, addr.clone())? {
        let cur_period = get_period(env.block.time.seconds())?;

        let last_checkpoint = fetch_last_checkpoint(deps.storage, &addr, cur_period)?;
        // The voting power point at the specified `time` was found
        let (voting_power, slope, fixed_amount) =
            if let Some(point) = last_checkpoint.map(|(_, point)| point) {
                if point.start == cur_period {
                    (point.power, point.slope, point.fixed)
                } else {
                    // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
                    (calc_voting_power(&point, cur_period), point.slope, point.fixed)
                }
            } else {
                (Uint128::zero(), Uint128::zero(), Uint128::zero())
            };

        let coefficient = calc_coefficient(lock.end - lock.last_extend_lock_period);

        let resp = LockInfoResponse {
            amount: lock.asset,
            coefficient,
            start: lock.start,
            end: lock.end,
            voting_power,
            fixed_amount,
            slope,
        };
        Ok(resp)
    } else {
        Err(ContractError::UserNotFound(addr.to_string()))
    }
}

/// Return a user's staked ampLP amount at a given block height.
///
/// * **user** user for which we return lock information.
///
/// * **block_height** block height at which we return the staked ampLP amount.
fn get_user_deposit_at_height(deps: Deps, user: String, block_height: u64) -> StdResult<Uint128> {
    let addr = deps.api.addr_validate(&user)?;
    let locked_opt = LOCKED.may_load_at_height(deps.storage, addr, block_height)?;
    if let Some(lock) = locked_opt {
        Ok(lock.asset)
    } else {
        Ok(Uint128::zero())
    }
}

/// Calculates a user's voting power at a given timestamp.
/// If time is None, then it calculates the user's voting power at the current block.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
///
/// * **time** timestamp at which to fetch the user's voting power (vAMP balance).
fn get_user_vamp(
    deps: Deps,
    env: Env,
    user: String,
    time: Option<u64>,
) -> StdResult<VotingPowerResponse> {
    let period = get_period(time.unwrap_or_else(|| env.block.time.seconds()))?;
    get_user_vamp_at_period(deps, user, period)
}

/// Calculates a user's voting power at a given period number.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
///
/// * **period** period number at which to fetch the user's voting power (vAMP balance).
fn get_user_vamp_at_period(
    deps: Deps,
    user: String,
    period: u64,
) -> StdResult<VotingPowerResponse> {
    let user = deps.api.addr_validate(&user)?;
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &user, period)?;

    if let Some(point) = last_checkpoint.map(|(_, point)| point) {
        // The voting power point at the specified `time` was found
        let voting_power = if point.start == period {
            point.power + point.fixed
        } else if point.end <= period {
            // the current period is after the voting end -> get default end power.
            point.fixed
        } else {
            // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
            calc_voting_power(&point, period) + point.fixed
        };
        Ok(VotingPowerResponse {
            vamp: voting_power,
        })
    } else {
        // User not found
        Ok(VotingPowerResponse {
            vamp: Uint128::zero(),
        })
    }
}

/// Calculates a user's voting power at the current block.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
fn get_user_balance(deps: Deps, env: Env, user: String) -> StdResult<BalanceResponse> {
    let vp_response = get_user_vamp(deps, env, user, None)?;
    Ok(BalanceResponse {
        balance: vp_response.vamp,
    })
}

/// Calculates the total voting power (total vAMP supply) at the given timestamp.
/// If `time` is None, then it calculates the total voting power at the current block.
///
/// * **time** timestamp at which we fetch the total voting power (vAMP supply).
fn get_total_vamp(deps: Deps, env: Env, time: Option<u64>) -> StdResult<VotingPowerResponse> {
    let period = get_period(time.unwrap_or_else(|| env.block.time.seconds()))?;
    get_total_vamp_at_period(deps, env, period)
}

/// Calculates the total voting power (total vAMP supply) at the given period number.
///
/// * **period** period number at which we fetch the total voting power (vAMP supply).
fn get_total_vamp_at_period(deps: Deps, env: Env, period: u64) -> StdResult<VotingPowerResponse> {
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &env.contract.address, period)?;

    let point = last_checkpoint.map_or(
        Point {
            power: Uint128::zero(),
            start: period,
            end: period,
            slope: Default::default(),
            fixed: Uint128::zero(),
        },
        |(_, point)| point,
    );

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
        vamp: voting_power,
    })
}

/// Fetch the vAMP token information, such as the token name, symbol, decimals and total supply (total voting power).
fn query_token_info(deps: Deps, env: Env) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    let total_vp = get_total_vamp(deps, env, None)?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: total_vp.vamp,
    };
    Ok(res)
}
