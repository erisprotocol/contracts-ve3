// use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage, Uint128};
// use cw_storage_plus::Bound;

// use ve3_shared::helpers::bps::BasicPoints;
// use ve3_shared::{asset_gauge::VotedValidatorInfoResponse, helpers::governance::calc_voting_power};

// use crate::state::{
//     UserInfo, VotedInfo, ASSET_SLOPE_CHANGES, ASSET_VOTES, USER_INFO, USER_SLOPE_CHANGES,
// };

// /// The enum defines math operations with voting power and slope.
// #[derive(Debug)]
// pub(crate) enum Operation {
//     Add,
//     Sub,
// }

// impl Operation {
//     pub fn calc(&self, cur_val: Uint128, amount: Uint128, bps: BasicPoints) -> Uint128 {
//         match self {
//             Operation::Add => cur_val + bps * amount,
//             Operation::Sub => cur_val.saturating_sub(bps * amount),
//         }
//     }
// }

// /// Enum wraps [`VotedPoolInfo`] so the contract can leverage storage operations efficiently.
// #[derive(Debug)]
// pub(crate) enum VotedPoolInfoResult {
//     Unchanged(VotedInfo),
//     New(VotedInfo),
// }

// /// Cancels user changes using old voting parameters for a given pool.
// /// Firstly, it removes slope change scheduled for previous lockup end period.
// /// Secondly, it updates voting parameters for the given period, but without user's vote.
// pub(crate) fn cancel_user_changes(
//     storage: &mut dyn Storage,
//     period: u64,
//     validator_addr: &str,
//     old_bps: BasicPoints,
//     old_vp: Uint128,
//     old_slope: Uint128,
//     old_lock_end: u64,
//     old_fixed_amount: Uint128,
// ) -> StdResult<()> {
//     // Cancel scheduled slope changes
//     let last_validator_period =
//         fetch_last_asset_period(storage, period, validator_addr)?.unwrap_or(period);
//     if last_validator_period < old_lock_end + 1 {
//         let end_period_key = old_lock_end + 1;
//         let old_scheduled_change =
//             ASSET_SLOPE_CHANGES.load(storage, (validator_addr, end_period_key))?;
//         let new_slope = old_scheduled_change.saturating_sub(old_bps * old_slope);
//         if !new_slope.is_zero() {
//             ASSET_SLOPE_CHANGES.save(storage, (validator_addr, end_period_key), &new_slope)?
//         } else {
//             ASSET_SLOPE_CHANGES.remove(storage, (validator_addr, end_period_key))
//         }
//     }

//     update_asset_info(
//         storage,
//         period,
//         validator_addr,
//         Some((old_bps, old_vp, old_slope, old_fixed_amount, Operation::Sub)),
//     )
//     .map(|_| ())
// }

// /// Applies user's vote for a given pool.
// /// Firstly, it schedules slope change for lockup end period.
// /// Secondly, it updates voting parameters with applied user's vote.
// pub(crate) fn vote_for_validator(
//     storage: &mut dyn Storage,
//     period: u64,
//     validator_addr: &str,
//     bps: BasicPoints,
//     vp: Uint128,
//     slope: Uint128,
//     fixed_amount: Uint128,
//     lock_end: u64,
// ) -> StdResult<()> {
//     // Schedule slope changes
//     ASSET_SLOPE_CHANGES.update::<_, StdError>(
//         storage,
//         (validator_addr, lock_end + 1),
//         |slope_opt| {
//             if let Some(saved_slope) = slope_opt {
//                 Ok(saved_slope + bps * slope)
//             } else {
//                 Ok(bps * slope)
//             }
//         },
//     )?;
//     update_asset_info(
//         storage,
//         period,
//         validator_addr,
//         Some((bps, vp, slope, fixed_amount, Operation::Add)),
//     )
//     .map(|_| ())
// }

// pub(crate) fn add_fixed_vamp(
//     storage: &mut dyn Storage,
//     period: u64,
//     validator_addr: &str,
//     vamps: Uint128,
// ) -> StdResult<()> {
//     let last = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;
//     let new = last.checked_add(vamps)?;
//     ASSET_FIXED_VAMP.save(storage, (validator_addr, period), &new)?;

//     Ok(())
// }

// pub(crate) fn remove_fixed_vamp(
//     storage: &mut dyn Storage,
//     period: u64,
//     validator_addr: &str,
//     vamps: Uint128,
// ) -> StdResult<()> {
//     // always change the future period only
//     let last = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;
//     let new = last
//         .checked_sub(vamps)
//         .map_err(|_| StdError::generic_err("remove_fixed_vamp: could not sub last with current"))?;
//     ASSET_FIXED_VAMP.save(storage, (validator_addr, period), &new)?;

//     Ok(())
// }

// /// Fetches voting parameters for a given pool at specific period, applies new changes, saves it in storage
// /// and returns new voting parameters in [`VotedPoolInfo`] object.
// /// If there are no changes in 'changes' parameter
// /// and voting parameters were already calculated before the function just returns [`VotedPoolInfo`].
// pub(crate) fn update_user_info(
//     storage: &mut dyn Storage,
//     period: u64,
//     user_addr: &Addr,
//     changes: Option<(Uint128, Uint128, Uint128, Operation)>,
// ) -> StdResult<VotedInfo> {
//     let period_key = period;
//     let validator_info = match get_validator_info_mut(storage, period, validator_addr)? {
//         VotedPoolInfoResult::Unchanged(mut validator_info)
//         | VotedPoolInfoResult::New(mut validator_info)
//             if changes.is_some() =>
//         {
//             if let Some((bps, vp, slope, fixed, op)) = changes {
//                 validator_info.slope = op.calc(validator_info.slope, slope, bps);
//                 validator_info.voting_power = op.calc(validator_info.voting_power, vp, bps);
//                 validator_info.fixed_amount = op.calc(validator_info.fixed_amount, fixed, bps)
//             }
//             ASSET_PERIODS.save(storage, (validator_addr, period_key), &())?;
//             ASSET_VOTES.save(storage, (period_key, validator_addr), &validator_info)?;
//             validator_info
//         },
//         VotedPoolInfoResult::New(validator_info) => {
//             ASSET_PERIODS.save(storage, (validator_addr, period_key), &())?;
//             ASSET_VOTES.save(storage, (period_key, validator_addr), &validator_info)?;
//             validator_info
//         },
//         VotedPoolInfoResult::Unchanged(validator_info) => validator_info,
//     };

//     Ok(validator_info)
// }

// /// Returns pool info at specified period or calculates it. Saves intermediate results in storage.
// pub(crate) fn get_validator_info_mut(
//     storage: &mut dyn Storage,
//     period: u64,
//     validator_addr: &str,
// ) -> StdResult<VotedPoolInfoResult> {
//     let validator_info_result = if let Some(validator_info) =
//         ASSET_VOTES.may_load(storage, (period, validator_addr))?
//     {
//         VotedPoolInfoResult::Unchanged(validator_info)
//     } else {
//         let validator_info_result = if let Some(mut prev_period) =
//             fetch_last_validator_period(storage, period, validator_addr)?
//         {
//             let mut validator_info = ASSET_VOTES.load(storage, (prev_period, validator_addr))?;
//             // Recalculating passed periods
//             let scheduled_slope_changes =
//                 fetch_slope_changes(storage, validator_addr, prev_period, period)?;
//             for (recalc_period, scheduled_change) in scheduled_slope_changes {
//                 validator_info = VotedInfo {
//                     voting_power: calc_voting_power(
//                         validator_info.slope,
//                         validator_info.voting_power,
//                         prev_period,
//                         recalc_period,
//                     ),
//                     slope: validator_info.slope.saturating_sub(scheduled_change),
//                     fixed_amount: validator_info.fixed_amount,
//                 };
//                 // Save intermediate result
//                 let recalc_period_key = recalc_period;
//                 ASSET_PERIODS.save(storage, (validator_addr, recalc_period_key), &())?;
//                 ASSET_VOTES.save(storage, (recalc_period_key, validator_addr), &validator_info)?;
//                 prev_period = recalc_period
//             }

//             VotedInfo {
//                 voting_power: calc_voting_power(
//                     validator_info.slope,
//                     validator_info.voting_power,
//                     prev_period,
//                     period,
//                 ),
//                 ..validator_info
//             }
//         } else {
//             VotedInfo::default()
//         };

//         VotedPoolInfoResult::New(validator_info_result)
//     };

//     Ok(validator_info_result)
// }

// /// Fetches voting parameters for a given pool at specific period, applies new changes, saves it in storage
// /// and returns new voting parameters in [`VotedPoolInfo`] object.
// /// If there are no changes in 'changes' parameter
// /// and voting parameters were already calculated before the function just returns [`VotedPoolInfo`].
// pub(crate) fn update_asset_info(
//     storage: &mut dyn Storage,
//     period: u64,
//     asset: &str,
//     changes: Option<(BasicPoints, Uint128, Uint128, Uint128, Operation)>,
// ) -> StdResult<VotedInfo> {
//     let period_key = period;
//     let validator_info = match get_asset_info_mut(storage, period, asset)? {
//         VotedPoolInfoResult::Unchanged(mut validator_info)
//         | VotedPoolInfoResult::New(mut validator_info)
//             if changes.is_some() =>
//         {
//             if let Some((bps, vp, slope, fixed, op)) = changes {
//                 validator_info.slope = op.calc(validator_info.slope, slope, bps);
//                 validator_info.voting_power = op.calc(validator_info.voting_power, vp, bps);
//                 validator_info.fixed_amount = op.calc(validator_info.fixed_amount, fixed, bps)
//             }
//             ASSET_VOTES.save(storage, (asset, period_key), &validator_info)?;
//             validator_info
//         },
//         VotedPoolInfoResult::New(validator_info) => {
//             ASSET_VOTES.save(storage, (asset, period_key), &validator_info)?;
//             validator_info
//         },
//         VotedPoolInfoResult::Unchanged(validator_info) => validator_info,
//     };

//     Ok(validator_info)
// }

// /// Returns pool info at specified period or calculates it. Saves intermediate results in storage.
// pub(crate) fn get_asset_info_mut(
//     storage: &mut dyn Storage,
//     period: u64,
//     asset: &str,
// ) -> StdResult<VotedPoolInfoResult> {
//     let validator_info_result = if let Some(validator_info) =
//         ASSET_VOTES.may_load(storage, (asset, period))?
//     {
//         VotedPoolInfoResult::Unchanged(validator_info)
//     } else {
//         let validator_info_result = if let Some((mut prev_period, mut validator_info)) =
//             fetch_last_asset_period(storage, period, asset)?
//         {
//             // let mut validator_info = ASSET_VOTES.load(storage, (prev_period, validator_addr))?;
//             // Recalculating passed periods
//             let scheduled_slope_changes = fetch_slope_changes(storage, asset, prev_period, period)?;
//             for (recalc_period, scheduled_change) in scheduled_slope_changes {
//                 validator_info = VotedInfo {
//                     voting_power: calc_voting_power(
//                         validator_info.slope,
//                         validator_info.voting_power,
//                         prev_period,
//                         recalc_period,
//                     ),
//                     slope: validator_info.slope.saturating_sub(scheduled_change),
//                     fixed_amount: validator_info.fixed_amount,
//                 };
//                 // Save intermediate result
//                 let recalc_period_key = recalc_period;
//                 ASSET_VOTES.save(storage, (recalc_period_key, asset), &validator_info)?;
//                 prev_period = recalc_period
//             }

//             VotedInfo {
//                 voting_power: calc_voting_power(
//                     validator_info.slope,
//                     validator_info.voting_power,
//                     prev_period,
//                     period,
//                 ),
//                 ..validator_info
//             }
//         } else {
//             VotedInfo::default()
//         };

//         VotedPoolInfoResult::New(validator_info_result)
//     };

//     Ok(validator_info_result)
// }

// /// Returns pool info at specified period or calculates it.
// pub(crate) fn get_asset_info(
//     storage: &dyn Storage,
//     period: u64,
//     asset: &str,
// ) -> StdResult<VotedValidatorInfoResponse> {
//     // let fixed_amount = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;

//     let validator_info =
//         if let Some(validator_info) = ASSET_VOTES.may_load(storage, (asset, period))? {
//             VotedValidatorInfoResponse {
//                 voting_power: validator_info.voting_power,
//                 slope: validator_info.slope,
//                 fixed_amount: validator_info.fixed_amount,
//             }
//         } else if let Some((mut prev_period, mut validator_info)) =
//             fetch_last_asset_period(storage, period, asset)?
//         {
//             // Recalculating passed periods
//             let scheduled_slope_changes = fetch_slope_changes(storage, asset, prev_period, period)?;
//             for (recalc_period, scheduled_change) in scheduled_slope_changes {
//                 validator_info = VotedInfo {
//                     voting_power: calc_voting_power(
//                         validator_info.slope,
//                         validator_info.voting_power,
//                         prev_period,
//                         recalc_period,
//                     ),
//                     slope: validator_info.slope.saturating_sub(scheduled_change),
//                     fixed_amount: validator_info.fixed_amount,
//                 };
//                 prev_period = recalc_period
//             }

//             VotedValidatorInfoResponse {
//                 voting_power: calc_voting_power(
//                     validator_info.slope,
//                     validator_info.voting_power,
//                     prev_period,
//                     period,
//                 ),
//                 fixed_amount: validator_info.fixed_amount,
//                 slope: validator_info.slope,
//             }
//         } else {
//             VotedValidatorInfoResponse::default()
//         };

//     Ok(validator_info)
// }

// /// Fetches last period for specified pool which has saved result in [`VALIDATOR_PERIODS`].
// pub(crate) fn fetch_last_asset_period(
//     storage: &dyn Storage,
//     period: u64,
//     asset: &str,
// ) -> StdResult<Option<(u64, VotedInfo)>> {
//     let period_opt = ASSET_VOTES
//         .prefix(asset)
//         .range(storage, None, Some(Bound::exclusive(period)), Order::Descending)
//         .next()
//         .transpose()?
//         // .map(|(period, _)| period)
//         ;
//     Ok(period_opt)
// }

// pub(crate) fn fetch_last_user_period(
//     storage: &dyn Storage,
//     period: u64,
//     user: &Addr,
// ) -> StdResult<Option<(u64, UserInfo)>> {
//     let period_opt = USER_INFO
//         .prefix(user)
//         .range(storage, None, Some(Bound::exclusive(period)), Order::Descending)
//         .next()
//         .transpose()?
//         // .map(|(period, _)| period)
//         ;
//     Ok(period_opt)
// }

// pub(crate) fn fetch_last_validator_fixed_vamp_value(
//     storage: &dyn Storage,
//     period: u64,
//     validator_addr: &str,
// ) -> StdResult<Uint128> {
//     let result = fetch_last_validator_fixed_vamp(storage, period, validator_addr)?;
//     Ok(result.unwrap_or_default())
// }

// pub(crate) fn fetch_last_validator_fixed_vamp(
//     storage: &dyn Storage,
//     period: u64,
//     validator_addr: &str,
// ) -> StdResult<Option<Uint128>> {
//     let emps_opt = ASSET_FIXED_VAMP
//         .prefix(validator_addr)
//         .range(storage, None, Some(Bound::inclusive(period)), Order::Descending)
//         .next()
//         .transpose()?
//         .map(|(_, emps)| emps);
//     Ok(emps_opt)
// }

// /// Fetches all slope changes between `last_period` and `period` for specific pool.
// pub(crate) fn fetch_slope_changes(
//     storage: &dyn Storage,
//     validator_addr: &str,
//     last_period: u64,
//     period: u64,
// ) -> StdResult<Vec<(u64, Uint128)>> {
//     ASSET_SLOPE_CHANGES
//         .prefix(validator_addr)
//         .range(
//             storage,
//             Some(Bound::exclusive(last_period)),
//             Some(Bound::inclusive(period)),
//             Order::Ascending,
//         )
//         .collect()
// }

// pub(crate) fn fetch_user_slope_changes(
//     storage: &dyn Storage,
//     user: &Addr,
//     last_period: u64,
//     period: u64,
// ) -> StdResult<Vec<(u64, Uint128)>> {
//     USER_SLOPE_CHANGES
//         .prefix(user)
//         .range(
//             storage,
//             Some(Bound::exclusive(last_period)),
//             Some(Bound::inclusive(period)),
//             Order::Ascending,
//         )
//         .collect()
// }
