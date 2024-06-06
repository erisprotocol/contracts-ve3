// use crate::state::{UserInfo, CONFIG, TUNE_INFO};
// use crate::utils::get_asset_info;
// #[cfg(not(feature = "library"))]
// use cosmwasm_std::entry_point;
// use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, Order, StdError, StdResult};
// use cw_storage_plus::Bound;
// use ve3_shared::asset_gauge::{
//     QueryMsg, UserInfoResponse, UserInfosResponse, VotedValidatorInfoResponse,
// };
// use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT};
// use ve3_shared::helpers::governance::get_period;

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//         QueryMsg::UserInfo {
//             user,
//         } => to_json_binary(&user_info(deps, env, user)?),
//         QueryMsg::UserInfos {
//             start_after,
//             limit,
//         } => to_json_binary(&user_infos(deps, env, start_after, limit)?),
//         QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
//         QueryMsg::ValidatorInfo {
//             validator_addr,
//         } => to_json_binary(&validator_info(deps, env, validator_addr, None)?),
//         QueryMsg::ValidatorInfos {
//             period,
//             validator_addrs,
//         } => to_json_binary(&validator_infos(deps, env, validator_addrs, period)?),
//         QueryMsg::ValidatorInfoAtPeriod {
//             validator_addr,
//             period,
//         } => to_json_binary(&validator_info(deps, env, validator_addr, Some(period))?),
//     }
// }

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
