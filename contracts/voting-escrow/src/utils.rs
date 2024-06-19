use crate::error::ContractError;
use crate::state::{Point, BLACKLIST, HISTORY, LAST_SLOPE_CHANGE, SLOPE_CHANGES};
use cosmwasm_std::{Addr, Coin, MessageInfo, Order, StdResult, Storage, Uint128};
use cw_asset::{Asset, AssetInfo};
use cw_storage_plus::Bound;
use ve3_shared::msgs_voting_escrow::End;
use ve3_shared::{
  constants::{MAX_LOCK_TIME, MIN_LOCK_PERIODS, SECONDS_PER_WEEK},
  extensions::asset_info_ext::AssetInfoExt,
  msgs_voting_escrow::{AssetInfoConfig, Config, DepositAsset},
};

/// Checks that a timestamp is within limits.
pub(crate) fn assert_time_limits(time: Option<u64>) -> Result<(), ContractError> {
  if let Some(time) = time {
    if !(SECONDS_PER_WEEK..=MAX_LOCK_TIME).contains(&time) {
      return Err(ContractError::LockTimeLimitsError {});
    }
  }
  Ok(())
}

pub(crate) fn assert_periods_remaining(end: &End, start: u64) -> Result<(), ContractError> {
  if let End::Period(end) = end {
    let periods = end - start;
    if periods < MIN_LOCK_PERIODS {
      return Err(ContractError::LockPeriodsError {});
    }
  }

  Ok(())
}

pub(crate) fn assert_not_decommissioned(config: &Config) -> Result<(), ContractError> {
  match config.decommissioned {
    Some(true) => Err(ContractError::DecommissionedError {}),
    _ => Ok(()),
  }
}

pub(crate) fn assert_asset_allowed(
  config: &Config,
  asset: &Asset,
) -> Result<AssetInfoConfig, ContractError> {
  if asset.amount.is_zero() {
    return Err(ContractError::RequiresAmount);
  }

  if let Some(asset) = config.deposit_assets.iter().find(|a| a.info == asset.info) {
    Ok(asset.config.clone())
  } else {
    Err(ContractError::WrongAsset(asset.info.to_string()))
  }
}

/// Checks if the blacklist contains a specific address.
pub(crate) fn assert_not_blacklisted(
  storage: &dyn Storage,
  addr: &Addr,
) -> Result<(), ContractError> {
  let blacklist = BLACKLIST.load(storage)?;
  if blacklist.contains(addr) {
    Err(ContractError::AddressBlacklisted(addr.to_string()))
  } else {
    Ok(())
  }
}

/// Checks if the blacklist contains a specific address.
pub(crate) fn assert_not_blacklisted_all(
  storage: &dyn Storage,
  addrs: Vec<Addr>,
) -> Result<(), ContractError> {
  let blacklist = BLACKLIST.load(storage)?;
  for addr in addrs {
    if blacklist.contains(&addr) {
      return Err(ContractError::AddressBlacklisted(addr.to_string()));
    }
  }
  Ok(())
}

/// Find the amount of a denom sent along a message, assert it is non-zero, and no other denom were
/// sent together
pub fn validate_received_funds(
  funds: &[Coin],
  allowed: &[DepositAsset<Addr>],
) -> Result<Asset, ContractError> {
  if funds.len() != 1 {
    return Err(ContractError::NoAssetsSent);
  }

  let fund = &funds[0];
  let info = AssetInfo::native(fund.denom.clone());
  let is_allowed = allowed.iter().any(|a| a.info == info);

  if !is_allowed {
    return Err(ContractError::WrongAsset(fund.denom.clone()));
  }

  if fund.amount.is_zero() {
    return Err(ContractError::RequiresAmount);
  }

  Ok(info.with_balance(fund.amount))
}

pub fn validate_received_cw20(
  asset: Asset,
  allowed: &[DepositAsset<Addr>],
) -> Result<Asset, ContractError> {
  let is_allowed = allowed.iter().any(|a| a.info == asset.info);

  if !is_allowed {
    return Err(ContractError::WrongAsset(asset.info.to_string()));
  }

  if asset.amount.is_zero() {
    return Err(ContractError::RequiresAmount);
  }

  Ok(asset)
}

/// Main function used to calculate a user's voting power at a specific period as: previous_power - slope*(x - previous_x).
pub(crate) fn calc_voting_power(point: &Point, period: u64) -> Uint128 {
  let shift = point
    .slope
    .checked_mul(Uint128::from(period - point.start))
    .unwrap_or_else(|_| Uint128::zero());
  point.power.checked_sub(shift).unwrap_or_else(|_| Uint128::zero())
}

/// Fetches the last checkpoint in [`HISTORY`] for the given address.
pub(crate) fn fetch_last_checkpoint(
  storage: &dyn Storage,
  token_id: &str,
  period_key: u64,
) -> StdResult<Option<(u64, Point)>> {
  HISTORY
    .prefix(token_id)
    .range(storage, None, Some(Bound::inclusive(period_key)), Order::Descending)
    .next()
    .transpose()
}

/// Cancels scheduled slope change of total voting power only if the given period is in future.
/// Removes scheduled slope change if it became zero.
pub(crate) fn cancel_scheduled_slope(
  storage: &mut dyn Storage,
  slope: Uint128,
  end: &End,
) -> StdResult<Option<(u64, u64)>> {
  if let End::Period(end) = end {
    let end = *end;
    let last_slope_change = LAST_SLOPE_CHANGE.may_load(storage)?.unwrap_or(0);

    // We do not need to schedule a slope change in the past
    if end > last_slope_change {
      match SLOPE_CHANGES.may_load(storage, end)? {
        Some(old_scheduled_change) => {
          let new_slope = old_scheduled_change.saturating_sub(slope);
          if !new_slope.is_zero() {
            SLOPE_CHANGES.save(storage, end, &(old_scheduled_change - slope))?;
          } else {
            SLOPE_CHANGES.remove(storage, end);
          }

          Ok(Some((last_slope_change, end)))
        },
        _ => Ok(Some((last_slope_change, end))),
      }
    } else {
      Ok(Some((last_slope_change, end)))
    }
  } else {
    Ok(None)
  }
}

/// Schedules slope change of total voting power in the given period.
pub(crate) fn schedule_slope_change(
  storage: &mut dyn Storage,
  slope: Uint128,
  period: &End,
) -> StdResult<()> {
  if !slope.is_zero() {
    if let End::Period(period) = period {
      SLOPE_CHANGES.update(storage, *period, |slope_opt| -> StdResult<Uint128> {
        if let Some(pslope) = slope_opt {
          Ok(pslope + slope)
        } else {
          Ok(slope)
        }
      })?;
      return Ok(());
    }
  }
  Ok(())
}

/// Fetches all slope changes between `last_slope_change` and `period`.
pub(crate) fn fetch_slope_changes(
  storage: &dyn Storage,
  last_slope_change: u64,
  period: u64,
) -> StdResult<Vec<(u64, Uint128)>> {
  SLOPE_CHANGES
    .range(
      storage,
      Some(Bound::exclusive(last_slope_change)),
      Some(Bound::inclusive(period)),
      Order::Ascending,
    )
    .collect()
}

pub(crate) fn message_info(sender: Addr) -> MessageInfo {
  MessageInfo {
    sender,
    funds: vec![],
  }
}
