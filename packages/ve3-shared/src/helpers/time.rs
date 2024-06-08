use crate::helpers::governance::get_period;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Env, StdResult};

#[cw_serde]
#[derive(Default)]
pub enum Time {
  #[default]
  Current,
  Next,
  Time(u64),
  Period(u64),
}

pub trait GetPeriod {
  fn get_period(self, env: &Env) -> StdResult<u64>;
}
impl GetPeriod for Option<Time> {
  fn get_period(self, env: &Env) -> StdResult<u64> {
    match self {
      Some(time) => match time {
        Time::Current => get_period(env.block.time.seconds()),
        Time::Next => Ok(get_period(env.block.time.seconds())? + 1),
        Time::Time(time) => get_period(time),
        Time::Period(period) => Ok(period),
      },
      None => get_period(env.block.time.seconds()),
    }
  }
}

#[cw_serde]
#[derive(Default)]
pub enum Times {
  #[default]
  Current,
  Times(Vec<u64>),
  Periods(Vec<u64>),
}

pub trait GetPeriods {
  fn get_periods(self, env: &Env) -> StdResult<Vec<u64>>;
}
impl GetPeriods for Option<Times> {
  fn get_periods(self, env: &Env) -> StdResult<Vec<u64>> {
    match self {
      Some(time) => match time {
        Times::Current => Ok(vec![get_period(env.block.time.seconds())?]),
        Times::Times(time) => time.into_iter().map(get_period).collect::<StdResult<Vec<_>>>(),
        Times::Periods(periods) => Ok(periods),
      },
      None => Ok(vec![get_period(env.block.time.seconds())?]),
    }
  }
}
