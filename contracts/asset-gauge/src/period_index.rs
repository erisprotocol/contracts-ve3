use std::fmt::Debug;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Order, OverflowError, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Map, Prefix};
use ve3_shared::{
  helpers::{bps::BasicPoints, governance::calc_voting_power},
  msgs_voting_escrow::LockInfoResponse,
};

pub struct PeriodIndex<'a> {
  pub data: Map<'a, (&'a str, u64), Data>,
  pub slope_changes: Map<'a, (&'a str, u64), Uint128>,
  pub keys: Map<'a, &'a str, ()>,
}

#[cw_serde]
#[derive(Default)]
pub struct Data {
  pub voting_power: Uint128,
  pub slope: Uint128,
  pub fixed_amount: Uint128,
}

impl Data {
  pub fn has_vp(&self) -> bool {
    !self.fixed_amount.is_zero() || !self.voting_power.is_zero()
  }

  pub fn total_vp(&self) -> Result<Uint128, OverflowError> {
    self.fixed_amount.checked_add(self.voting_power)
  }
}

#[cw_serde]
pub struct UserExtension {
  pub votes: Vec<(String, BasicPoints)>,
}

/// Enum wraps [`VotedPoolInfo`] so the contract can leverage storage operations efficiently.
#[derive(Debug)]
pub enum VoteResult {
  Unchanged(Data),
  New(Data),
}

/// The enum defines math operations with voting power and slope.
#[derive(Debug)]
pub enum Operation {
  Add,
  Sub,
}

impl Operation {
  pub fn calc(&self, cur_val: Uint128, amount: Uint128, bps: BasicPoints) -> Uint128 {
    match self {
      Operation::Add => cur_val + bps * amount,
      Operation::Sub => cur_val.saturating_sub(bps * amount),
    }
  }
}

#[cw_serde]
pub struct Line {
  vp: Uint128,
  slope: Uint128,
  fixed: Uint128,
  start: u64,
  end: u64,
}

impl From<&LockInfoResponse> for Line {
  fn from(val: &LockInfoResponse) -> Self {
    Line {
      vp: val.voting_power,
      slope: val.slope,
      fixed: val.fixed_amount,
      start: val.start,
      end: val.end,
    }
  }
}

impl<'a> PeriodIndex<'a> {
  pub fn new(data_key: &'a str, slope_key: &'a str, keys_key: &'a str) -> Self {
    Self {
      data: Map::new(data_key),
      slope_changes: Map::new(slope_key),
      keys: Map::new(keys_key),
    }
  }

  pub fn clear(&self, storage: &mut dyn Storage, limit: Option<usize>) {
    Prefix::<Vec<u8>, (), &str>::new(self.keys.namespace(), &[]).clear(storage, limit);
    Prefix::<Vec<u8>, Data, (&str, u64)>::new(self.data.namespace(), &[]).clear(storage, limit);
    Prefix::<Vec<u8>, Uint128, (&str, u64)>::new(self.slope_changes.namespace(), &[])
      .clear(storage, limit);
  }

  /// Applies user's vote for a given pool.   
  /// Firstly, it schedules slope change for lockup end period.  
  /// Secondly, it updates voting parameters with applied user's vote.
  pub fn add_vote(
    &self,
    storage: &mut dyn Storage,
    period: u64,
    key: &str,
    bps: BasicPoints,
    line: Line,
  ) -> StdResult<Data> {
    let vp = line.vp;
    let slope = line.slope;
    let fixed_amount = line.fixed;
    let lock_end = line.end;

    if (!fixed_amount.is_zero() || !vp.is_zero()) && !self.keys.has(storage, key) {
      self.keys.save(storage, key, &())?;
    }

    // Schedule slope changes
    self.slope_changes.update::<_, StdError>(storage, (key, lock_end + 1), |slope_opt| {
      if let Some(saved_slope) = slope_opt {
        Ok(saved_slope + bps * slope)
      } else {
        Ok(bps * slope)
      }
    })?;
    let data = self.update_data(
      storage,
      period,
      key,
      Some((bps, vp, slope, fixed_amount, Operation::Add)),
    )?;

    Ok(data)
  }

  #[cfg(test)]
  pub fn add_0(
    &self,
    storage: &mut dyn Storage,
    period: u64,
    key: &str,
    bps: BasicPoints,
    line: Line,
  ) -> StdResult<&PeriodIndex<'a>> {
    self.add_vote(storage, period, key, bps, line)?;
    Ok(self)
  }

  /// Cancels user changes using old voting parameters for a given pool.  
  /// Firstly, it removes slope change scheduled for previous lockup end period.  
  /// Secondly, it updates voting parameters for the given period, but without user's vote.
  pub(crate) fn remove_vote(
    &self,
    storage: &mut dyn Storage,
    // block +1
    period: u64,
    key: &str,
    old_bps: BasicPoints,
    line: Line,
  ) -> StdResult<Data> {
    let old_slope = line.slope;
    let old_fixed_amount = line.fixed;
    let old_lock_end = line.end;

    // Cancel scheduled slope changes
    let (last_validator_period, _) =
      self.fetch_last_period(storage, period, key)?.unwrap_or((period, Data::default()));

    if last_validator_period < old_lock_end + 1 {
      let end_period_key = old_lock_end + 1;
      let old_scheduled_change = self.slope_changes.load(storage, (key, end_period_key))?;
      let new_slope = old_scheduled_change.saturating_sub(old_bps * old_slope);
      if !new_slope.is_zero() {
        self.slope_changes.save(storage, (key, end_period_key), &new_slope)?
      } else {
        self.slope_changes.remove(storage, (key, end_period_key))
      }
    }

    // this is the remaining vp
    let vp_to_reduce = if old_lock_end + 1 > period {
      old_slope
        .checked_mul(Uint128::from(old_lock_end + 1 - period))
        .unwrap_or_else(|_| Uint128::zero())
    } else {
      Uint128::zero()
    };

    // println!("remove {:?}", line);
    // println!("old_lock_end {:?}", old_lock_end);
    // println!("period {:?}", period);
    // println!("vp_to_reduce {:?}", vp_to_reduce);

    let slope_to_reduce = if old_lock_end + 1 > period {
      old_slope
    } else {
      Uint128::zero()
    };

    let result = self.update_data(
      storage,
      period,
      key,
      Some((old_bps, vp_to_reduce, slope_to_reduce, old_fixed_amount, Operation::Sub)),
    )?;

    if result.fixed_amount.is_zero() && !result.voting_power.is_zero() {
      self.keys.remove(storage, key);
    }

    Ok(result)
  }

  #[cfg(test)]
  pub(crate) fn remove_vote_0(
    &self,
    storage: &mut dyn Storage,
    // block +1
    period: u64,
    key: &str,
    old_bps: BasicPoints,
    line: Line,
  ) -> StdResult<&PeriodIndex<'a>> {
    self.remove_vote(storage, period, key, old_bps, line)?;
    Ok(self)
  }

  #[allow(clippy::too_many_arguments)]
  pub(crate) fn change_vote(
    &self,
    storage: &mut dyn Storage,
    // block +1
    period: u64,
    key: &str,
    old_bps: BasicPoints,
    new_bps: BasicPoints,
    current: &Data,
    slopes: &[(u64, Uint128)],
  ) -> StdResult<Data> {
    let slope = current.slope;
    let vp = current.voting_power;
    let fixed = current.fixed_amount;
    let period_key = period;

    let result = match self.get_asset_info_mut(storage, period, key)? {
      VoteResult::Unchanged(mut validator_info) | VoteResult::New(mut validator_info)
        if (!old_bps.is_zero() || !new_bps.is_zero()) && old_bps != new_bps =>
      {
        if !old_bps.is_zero() {
          let op = Operation::Sub;
          validator_info.slope = op.calc(validator_info.slope, slope, old_bps);
          validator_info.voting_power = op.calc(validator_info.voting_power, vp, old_bps);
          validator_info.fixed_amount = op.calc(validator_info.fixed_amount, fixed, old_bps)
        }

        if !new_bps.is_zero() {
          let op = Operation::Add;
          validator_info.slope = op.calc(validator_info.slope, slope, new_bps);
          validator_info.voting_power = op.calc(validator_info.voting_power, vp, new_bps);
          validator_info.fixed_amount = op.calc(validator_info.fixed_amount, fixed, new_bps)
        }

        self.data.save(storage, (key, period_key), &validator_info)?;
        validator_info
      },
      VoteResult::New(validator_info) => {
        self.data.save(storage, (key, period_key), &validator_info)?;
        validator_info
      },
      VoteResult::Unchanged(validator_info) => validator_info,
    };

    if (!old_bps.is_zero() || !new_bps.is_zero()) && old_bps != new_bps {
      // iterate slopes and apply changes to the asset
      for (period, slope) in slopes.iter().copied() {
        let mut current = self.slope_changes.may_load(storage, (key, period))?.unwrap_or_default();
        if !old_bps.is_zero() {
          let op = Operation::Sub;
          current = op.calc(current, slope, old_bps);
        }
        if !new_bps.is_zero() {
          let op = Operation::Add;
          current = op.calc(current, slope, new_bps);
        }

        if current.is_zero() {
          self.slope_changes.remove(storage, (key, period));
        } else {
          self.slope_changes.save(storage, (key, period), &current)?;
        }
      }
    }

    if result.has_vp() {
      if !self.keys.has(storage, key) {
        self.keys.save(storage, key, &())?;
      }
    } else {
      self.keys.remove(storage, key);
    }

    Ok(result)
  }

  #[cfg(test)]
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn change_0(
    &self,
    storage: &mut dyn Storage,
    // block +1
    period: u64,
    key: &str,
    old_bps: BasicPoints,
    new_bps: BasicPoints,

    current: &Data,
    slopes: &[(u64, Uint128)],
  ) -> StdResult<&PeriodIndex<'a>> {
    self.change_vote(storage, period, key, old_bps, new_bps, current, slopes)?;
    Ok(self)
  }

  /// Fetches voting parameters for a given pool at specific period, applies new changes, saves it in storage
  /// and returns new voting parameters in [`VotedPoolInfo`] object.
  /// If there are no changes in 'changes' parameter
  /// and voting parameters were already calculated before the function just returns [`VotedPoolInfo`].
  pub fn update_data(
    &self,
    storage: &mut dyn Storage,
    period: u64,
    key: &str,
    changes: Option<(BasicPoints, Uint128, Uint128, Uint128, Operation)>,
  ) -> StdResult<Data> {
    let period_key = period;
    let validator_info = match self.get_asset_info_mut(storage, period, key)? {
      VoteResult::Unchanged(mut validator_info) | VoteResult::New(mut validator_info)
        if changes.is_some() =>
      {
        if let Some((bps, vp, slope, fixed, op)) = changes {
          validator_info.slope = op.calc(validator_info.slope, slope, bps);
          validator_info.voting_power = op.calc(validator_info.voting_power, vp, bps);
          validator_info.fixed_amount = op.calc(validator_info.fixed_amount, fixed, bps)
        }

        self.data.save(storage, (key, period_key), &validator_info)?;
        validator_info
      },
      VoteResult::New(validator_info) => {
        self.data.save(storage, (key, period_key), &validator_info)?;
        validator_info
      },
      VoteResult::Unchanged(validator_info) => validator_info,
    };

    Ok(validator_info)
  }

  /// Returns pool info at specified period or calculates it. Saves intermediate results in storage.
  fn get_asset_info_mut(
    &self,
    storage: &mut dyn Storage,
    period: u64,
    key: &str,
  ) -> StdResult<VoteResult> {
    let validator_info_result =
      if let Some(validator_info) = self.data.may_load(storage, (key, period))? {
        VoteResult::Unchanged(validator_info)
      } else {
        let validator_info_result = if let Some((mut prev_period, mut validator_info)) =
          self.fetch_last_period(storage, period, key)?
        {
          // let mut validator_info = ASSET_VOTES.load(storage, (prev_period, validator_addr))?;
          // Recalculating passed periods
          let scheduled_slope_changes =
            self.fetch_slope_changes(storage, key, prev_period, period)?;
          for (recalc_period, scheduled_change) in scheduled_slope_changes {
            validator_info = Data {
              voting_power: calc_voting_power(
                validator_info.slope,
                validator_info.voting_power,
                prev_period,
                recalc_period,
              ),
              slope: validator_info.slope.saturating_sub(scheduled_change),
              fixed_amount: validator_info.fixed_amount,
            };
            // Save intermediate result
            let recalc_period_key = recalc_period;
            self.data.save(storage, (key, recalc_period_key), &validator_info)?;
            prev_period = recalc_period
          }

          Data {
            voting_power: calc_voting_power(
              validator_info.slope,
              validator_info.voting_power,
              prev_period,
              period,
            ),
            ..validator_info
          }
        } else {
          Data {
            voting_power: Uint128::zero(),
            slope: Uint128::zero(),
            fixed_amount: Uint128::zero(),
          }
        };

        VoteResult::New(validator_info_result)
      };

    Ok(validator_info_result)
  }

  pub fn get_latest_data(&self, storage: &dyn Storage, period: u64, key: &str) -> StdResult<Data> {
    // let fixed_amount = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;

    let validator_info = if let Some(validator_info) = self.data.may_load(storage, (key, period))? {
      validator_info
    } else if let Some((mut prev_period, mut validator_info)) =
      self.fetch_last_period(storage, period, key)?
    {
      // Recalculating passed periods
      let scheduled_slope_changes = self.fetch_slope_changes(storage, key, prev_period, period)?;
      for (recalc_period, scheduled_change) in scheduled_slope_changes {
        validator_info = Data {
          voting_power: calc_voting_power(
            validator_info.slope,
            validator_info.voting_power,
            prev_period,
            recalc_period,
          ),
          slope: validator_info.slope.saturating_sub(scheduled_change),
          fixed_amount: validator_info.fixed_amount,
        };
        prev_period = recalc_period
      }

      Data {
        voting_power: calc_voting_power(
          validator_info.slope,
          validator_info.voting_power,
          prev_period,
          period,
        ),
        fixed_amount: validator_info.fixed_amount,
        slope: validator_info.slope,
      }
    } else {
      Data::default()
    };

    Ok(validator_info)
  }

  fn fetch_slope_changes(
    &self,
    storage: &dyn Storage,
    key: &str,
    last_period: u64,
    period: u64,
  ) -> StdResult<Vec<(u64, Uint128)>> {
    self
      .slope_changes
      .prefix(key)
      .range(
        storage,
        Some(Bound::exclusive(last_period)),
        Some(Bound::inclusive(period)),
        Order::Ascending,
      )
      .collect()
  }

  pub fn fetch_future_slope_changes(
    &self,
    storage: &dyn Storage,
    key: &str,
    period: u64,
  ) -> StdResult<Vec<(u64, Uint128)>> {
    self
      .slope_changes
      .prefix(key)
      .range(storage, Some(Bound::inclusive(period)), None, Order::Ascending)
      .collect()
  }

  fn fetch_last_period(
    &self,
    storage: &dyn Storage,
    period: u64,
    key: &str,
  ) -> StdResult<Option<(u64, Data)>> {
    let period_opt = self.data
            .prefix(key)
            .range(storage, None, Some(Bound::exclusive(period)), Order::Descending)
            .next()
            .transpose()?
            // .map(|(period, _)| period)
            ;
    Ok(period_opt)
  }

  pub fn print(&self, storage: &mut dyn Storage, text: &str) -> &PeriodIndex<'a> {
    println!("Points {text}");
    for element in self.data.range(storage, None, None, Order::Ascending) {
      let ((key, period), data) = element.unwrap();
      println!("key {key} period {period} - {data:?}")
    }

    println!("Slopes");
    for element in self.slope_changes.range(storage, None, None, Order::Ascending) {
      let ((key, period), data) = element.unwrap();
      println!("key {key} period {period} - {data:?}")
    }

    println!("Keys");
    for element in self.keys.range(storage, None, None, Order::Ascending) {
      let (key, _) = element.unwrap();
      println!("existing key {key}")
    }

    println!("------ {text}");
    self
  }
}

#[cfg(test)]
mod test {
  use cosmwasm_std::{testing::mock_dependencies, StdResult, Uint128};
  use ve3_shared::helpers::bps::BasicPoints;

  use super::{Line, PeriodIndex};

  #[test]
  pub fn test_index() -> StdResult<()> {
    let mut mock = mock_dependencies();
    let deps = mock.as_mut();
    let index = PeriodIndex::new("data", "slope", "keys");

    let vp = Uint128::new(100_000000u128);
    let fixed = Uint128::new(10_000000u128);
    let slope = Uint128::new(1_000000u128);

    let user_1 = Line {
      vp,
      slope,
      fixed,
      start: 1,
      end: 101,
    };

    let user_2_vote_1 = Line {
      vp,
      slope,
      fixed,
      start: 4,
      end: 104,
    };

    let user_2_vote_2 = Line {
      vp: Uint128::new(10_000000),
      slope,
      fixed: Uint128::new(20_000000),
      start: 10,
      end: 20,
    };

    index
      .add_0(deps.storage, 1, "user1", BasicPoints::one(), user_1.clone())?
      .add_0(deps.storage, 1, "lp1", BasicPoints::percent(10), user_1.clone())?
      .add_0(deps.storage, 1, "lp2", BasicPoints::percent(90), user_1.clone())?
      .print(deps.storage, "add user_1")
      //
      .add_0(deps.storage, 4, "user2", BasicPoints::one(), user_2_vote_1.clone())?
      .add_0(deps.storage, 4, "lp2", BasicPoints::percent(80), user_2_vote_1.clone())?
      .add_0(deps.storage, 4, "lp3", BasicPoints::percent(20), user_2_vote_1.clone())?
      .print(deps.storage, "add user_2_vote_1")
      //
      .remove_vote_0(deps.storage, 5, "user1", BasicPoints::one(), user_1.clone())?
      .remove_vote_0(deps.storage, 5, "lp1", BasicPoints::percent(10), user_1.clone())?
      .remove_vote_0(deps.storage, 5, "lp2", BasicPoints::percent(90), user_1.clone())?
      .print(deps.storage, "remove user_1")
      //
      .add_0(deps.storage, 10, "user2", BasicPoints::one(), user_2_vote_2.clone())?
      .add_0(deps.storage, 10, "lp2", BasicPoints::percent(80), user_2_vote_2.clone())?
      .add_0(deps.storage, 10, "lp3", BasicPoints::percent(20), user_2_vote_2.clone())?
      .print(deps.storage, "add user_2_vote_2");

    // let result = index
    //     .remove_vote(deps.storage, 15, "lp2", Decimal::percent(70).try_into().unwrap(), user_2)
    //     .unwrap();

    Ok(())
  }

  #[test]
  pub fn test_index_2() -> StdResult<()> {
    let mut mock = mock_dependencies();
    let deps = mock.as_mut();
    let index = PeriodIndex::new("data", "slope", "keys");

    let vp = Uint128::new(100_000000u128);
    let fixed = Uint128::new(10_000000u128);
    let slope = Uint128::new(1_000000u128);

    let user_1_vote_1 = Line {
      vp,
      slope,
      fixed,
      start: 1,
      end: 101,
    };
    let user_1_vote_2 = Line {
      vp: Uint128::new(100_000000u128),
      slope: Uint128::new(10_000000u128),
      fixed: Uint128::new(100_000000u128),
      start: 4,
      end: 14,
    };

    index
      .add_0(deps.storage, 1, "user1", BasicPoints::one(), user_1_vote_1.clone())?
      .add_0(deps.storage, 1, "lp1", BasicPoints::percent(10), user_1_vote_1.clone())?
      .add_0(deps.storage, 1, "lp2", BasicPoints::percent(90), user_1_vote_1.clone())?
      .print(deps.storage, "add user_1_vote_1")
      .add_0(deps.storage, 4, "user1", BasicPoints::one(), user_1_vote_2.clone())?
      .add_0(deps.storage, 4, "lp1", BasicPoints::percent(10), user_1_vote_2.clone())?
      .add_0(deps.storage, 4, "lp2", BasicPoints::percent(90), user_1_vote_2.clone())?
      .print(deps.storage, "add user_1_vote_2");

    let period = 5;
    let slopes = index.fetch_future_slope_changes(deps.storage, "user1", period)?;
    let user = index.get_latest_data(deps.storage, period, "user1")?;
    println!("user {user:?}");

    index
      .change_0(
        deps.storage,
        period,
        "lp1",
        BasicPoints::percent(10),
        BasicPoints::percent(50),
        &user,
        &slopes,
      )?
      .change_0(
        deps.storage,
        period,
        "lp2",
        BasicPoints::percent(90),
        BasicPoints::percent(50),
        &user,
        &slopes,
      )?
      .print(deps.storage, "change vote 1");

    // moving lp1 to lp3, lp2 stays same

    let period = 10;
    let slopes = index.fetch_future_slope_changes(deps.storage, "user1", period)?;
    let user = index.get_latest_data(deps.storage, period, "user1")?;
    println!("user {user:?}");
    index
      .change_0(
        deps.storage,
        period,
        "lp1",
        BasicPoints::percent(50),
        BasicPoints::percent(0),
        &user,
        &slopes,
      )?
      .change_0(
        deps.storage,
        period,
        "lp2",
        BasicPoints::percent(50),
        BasicPoints::percent(50),
        &user,
        &slopes,
      )?
      .change_0(
        deps.storage,
        period,
        "lp3",
        BasicPoints::percent(0),
        BasicPoints::percent(50),
        &user,
        &slopes,
      )?
      .print(deps.storage, "change vote 2");

    let user = index.get_latest_data(deps.storage, 120, "user1")?;
    println!("user {user:?}");

    let lp1 = index.get_latest_data(deps.storage, 120, "lp1")?;
    println!("lp1 {lp1:?}");
    let lp2 = index.get_latest_data(deps.storage, 120, "lp2")?;
    println!("lp2 {lp2:?}");
    let lp3 = index.get_latest_data(deps.storage, 120, "lp3")?;
    println!("lp3 {lp3:?}");

    // let result = index
    //     .remove_vote(deps.storage, 15, "lp2", Decimal::percent(70).try_into().unwrap(), user_2)
    //     .unwrap();

    Ok(())
  }
}