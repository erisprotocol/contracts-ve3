use crate::error::ContractError;
use crate::state::{
  fetch_last_gauge_distribution, fetch_last_gauge_vote, user_idx, AssetIndex,
  GaugeDistributionPeriod, UserVotes, CONFIG, GAUGE_DISTRIBUTION, GAUGE_VOTE, LOCK_INFO,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  attr, Addr, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_asset::AssetInfo;
use itertools::Itertools;
use std::collections::HashSet;
use std::convert::TryInto;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::adapters::ve3_asset_staking::Ve3AssetStaking;
use ve3_shared::msgs_asset_gauge::{Config, ExecuteMsg, GaugeConfig, InstantiateMsg, MigrateMsg};
use ve3_shared::constants::{AT_GAUGE_CONTROLLER, AT_VOTING_ESCROW};
use ve3_shared::msgs_asset_staking::AssetDistribution;
use ve3_shared::helpers::bps::BasicPoints;
use ve3_shared::helpers::governance::get_period;
use ve3_shared::msgs_voting_escrow::LockInfoResponse;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  for gauge in msg.gauges.iter() {
    deps.api.addr_validate(gauge.target.as_str())?;
  }

  CONFIG.save(
    deps.storage,
    &Config {
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
      gauges: msg.gauges,
    },
  )?;

  Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  match msg {
    ExecuteMsg::Vote {
      gauge,
      votes,
    } => handle_vote(deps, env, info, gauge, votes),

    ExecuteMsg::UpdateVote {
      token_id,
      lock_info,
    } => update_vote(deps, env, info.sender, token_id, lock_info),

    ExecuteMsg::ClearGaugeState {
      gauge,
      limit,
    } => {
      let config = CONFIG.load(deps.storage)?;
      if config.gauges.iter().any(|a| a.name == gauge) {
        return Err(ContractError::CannotClearExistingGauge {});
      }
      AssetIndex::new(&gauge).idx().clear(deps.storage, limit);
      Ok(Response::default().add_attribute("action", "gauge/clear_gauge_state"))
    },

    ExecuteMsg::SetDistribution {} => set_distribution(deps, env, info),

    ExecuteMsg::UpdateConfig {
      update_gauge,
      remove_gauge,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(gauge) = update_gauge {
        deps.api.addr_validate(gauge.target.as_str())?;
        config.gauges.retain(|a| a.name != gauge.name);
        config.gauges.push(gauge);
      }

      if let Some(name) = remove_gauge {
        config.gauges.retain(|a| a.name != name);
      }

      CONFIG.save(deps.storage, &config)?;
      Ok(Response::default().add_attribute("action", "gauge/update_config"))
    },
  }
}

fn handle_vote(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  gauge: String,
  votes: Vec<(String, u16)>,
) -> Result<Response, ContractError> {
  let sender = info.sender;
  let gauge = &gauge;
  let block_period = get_period(env.block.time.seconds())?;
  let config = CONFIG.load(deps.storage)?;
  let gauge_config = config.assert_gauge(gauge)?;

  let user_index = user_idx();
  let asset_index = AssetIndex::new(gauge);
  let asset_index = asset_index.idx();

  let current_user = user_index.get_latest_data(deps.storage, block_period + 1, sender.as_str())?;
  if !current_user.has_vp() {
    return Err(ContractError::ZeroVotingPower(sender.to_string(), block_period + 1));
  }

  let (_, old_votes) =
    fetch_last_gauge_vote(deps.storage, gauge, sender.as_str(), block_period + 1)?
      .unwrap_or_default();

  let allowed = gauge_config.query_whitelisted_assets_str(&deps.querier)?;

  let mut values_set: HashSet<_> = HashSet::new();
  let mut changes =
    old_votes.votes.into_iter().map(|a| (a.0, a.1, BasicPoints::zero())).collect::<Vec<_>>();

  let votes = votes
    .into_iter()
    .map(|(addr, bps)| {
      if !values_set.insert(addr.clone()) {
        return Err(ContractError::DuplicatedVotes {});
      }
      if !allowed.contains(&addr) {
        return Err(ContractError::InvalidValidatorAddress(addr));
      }

      let bps: BasicPoints = bps.try_into()?;
      let old_vote = changes.iter_mut().find(|a| a.0 == addr);
      match old_vote {
        Some(found) => found.2 = bps,
        None => changes.push((addr.clone(), BasicPoints::zero(), bps)),
      }
      Ok((addr, bps))
    })
    .collect::<Result<Vec<_>, ContractError>>()?;

  // Check the bps sum is within the limit
  votes.iter().try_fold(BasicPoints::default(), |acc, (_, bps)| acc.checked_add(*bps))?;

  let slope_changes: Vec<(u64, Uint128)> =
    user_index.fetch_future_slope_changes(deps.storage, sender.as_str(), block_period + 1)?;

  for (asset, old, new) in changes {
    asset_index.change_vote(
      deps.storage,
      block_period + 1,
      &asset,
      old,
      new,
      &current_user,
      &slope_changes,
    )?;
  }

  GAUGE_VOTE.save(
    deps.storage,
    (&gauge, sender.as_str(), block_period + 1),
    &UserVotes {
      votes,
    },
  )?;

  Ok(
    Response::new()
      .add_attribute("action", "vegauge/vote")
      .add_attribute("vp", current_user.total_vp()?),
  )
}

fn remove_votes_of_user(
  storage: &mut dyn Storage,
  config: &Config,
  block_period: u64,
  old_lock: &LockInfoResponse,
) -> Result<(), ContractError> {
  let user = old_lock.owner.as_str();

  user_idx().remove_vote(storage, block_period + 1, user, BasicPoints::max(), old_lock.into())?;

  // Cancel changes applied by previous votes
  for gauge_config in config.gauges.iter() {
    let gauge = &gauge_config.name;
    let vote = fetch_last_gauge_vote(storage, gauge, user, block_period + 1)?;

    if let Some((_, votes)) = vote {
      let asset_index = AssetIndex::new(gauge);
      let asset_index = asset_index.idx();

      for (key, bps) in votes.votes {
        asset_index.remove_vote(storage, block_period + 1, &key, bps, old_lock.into())?;
      }
    }
  }

  Ok(())
}

fn apply_votes_of_user(
  storage: &mut dyn Storage,
  config: &Config,
  block_period: u64,
  new_lock: LockInfoResponse,
) -> Result<(), ContractError> {
  let user = new_lock.owner.as_str();

  user_idx().add_vote(storage, block_period + 1, user, BasicPoints::max(), (&new_lock).into())?;

  // Cancel changes applied by previous votes
  for gauge_config in config.gauges.iter() {
    let gauge = &gauge_config.name;
    let vote = fetch_last_gauge_vote(storage, gauge, user, block_period + 1)?;

    if let Some((_, votes)) = vote {
      let asset_index = AssetIndex::new(gauge);
      let asset_index = asset_index.idx();

      for (key, bps) in votes.votes {
        asset_index.add_vote(storage, block_period + 1, &key, bps, (&new_lock).into())?;
      }
    }
  }

  Ok(())
}

fn update_vote(
  deps: DepsMut,
  env: Env,
  sender: Addr,
  token_id: String,
  new_lock: LockInfoResponse,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  config.global_config().assert_has_access(&deps.querier, AT_VOTING_ESCROW, &sender)?;

  let block_period = get_period(env.block.time.seconds())?;

  let old_lock = LOCK_INFO.may_load(deps.storage, &token_id)?;
  LOCK_INFO.save(deps.storage, &token_id, &new_lock)?;

  if let Some(old_lock) = old_lock {
    remove_votes_of_user(deps.storage, &config, block_period, &old_lock)?;
  }
  if new_lock.has_vp() {
    apply_votes_of_user(deps.storage, &config, block_period, new_lock)?;
  }

  Ok(Response::new().add_attribute("action", "gauge/update_vote"))
}

/// The function checks that the last pools tuning happened >= 14 days ago.
/// Then it calculates voting power for each pool at the current period, filters all pools which
/// are not eligible to receive allocation points,
/// takes top X pools by voting power, where X is 'config.pools_limit', calculates allocation points
/// for these pools and applies allocation points in generator contract.
fn set_distribution(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  config.global_config().assert_has_access(&deps.querier, AT_GAUGE_CONTROLLER, &info.sender)?;
  let block_period = get_period(env.block.time.seconds())?;

  let mut attrs = vec![];
  let mut msgs = vec![];
  for gauge_config in config.gauges {
    let asset_staking = Ve3AssetStaking(gauge_config.target.clone());
    let assets = asset_staking.query_whitelisted_assets(&deps.branch().querier)?;
    let gauge = &gauge_config.name;
    let mut periods = vec![];

    let distribution =
      match fetch_last_gauge_distribution(deps.branch().storage, gauge, block_period)? {
        Some((last_period, _)) if last_period == block_period => None,

        Some((mut last_period, _)) => {
          while last_period < block_period {
            _set_distribution(deps.branch(), &env, &gauge_config, &assets, last_period)?;
            periods.push(last_period);
            last_period += 1;
          }

          periods.push(block_period);
          Some(_set_distribution(deps.branch(), &env, &gauge_config, &assets, block_period)?)
        },

        None => {
          periods.push(block_period);
          Some(_set_distribution(deps.branch(), &env, &gauge_config, &assets, block_period)?)
        },
      };

    attrs.push(attr("gauge", gauge_config.name));
    attrs.push(attr("periods", periods.iter().join(",")));

    if let Some(new_distribution) = distribution {
      msgs.push(asset_staking.set_reward_distribution_msg(new_distribution)?)
    }
  }

  Ok(
    Response::new()
      .add_attribute("action", "gauge/set_distribution")
      .add_attributes(attrs)
      .add_messages(msgs),
  )
}

fn _set_distribution(
  deps: DepsMut,
  _env: &Env,
  gauge_config: &GaugeConfig,
  assets: &[AssetInfo],
  period: u64,
) -> Result<Vec<AssetDistribution>, ContractError> {
  let gauge = &gauge_config.name;
  let asset_index = AssetIndex::new(gauge);
  let asset_index = asset_index.idx();

  let allowed_votes: Vec<_> = assets
    .iter()
    .map(|key| {
      let key_raw = &key.to_string();
      let vote_info = asset_index.update_data(deps.storage, period, key_raw, None)?;
      let vp = vote_info.total_vp()?;

      Ok((key.clone(), vp))
    })
    .collect::<StdResult<Vec<_>>>()?
    .into_iter()
    .filter(|(_, vp)| !vp.is_zero())
    .sorted_by(|(_, a), (_, b)| b.cmp(a)) // Sort in descending order
    .collect();

  let total_voting_power: Uint128 = allowed_votes.iter().map(|(_, b)| b).sum();
  let min_voting_power = gauge_config.min_gauge_percentage * total_voting_power;

  let relevant_votes =
    allowed_votes.into_iter().filter(|(_, amount)| *amount > min_voting_power).collect_vec();

  let sum_relevant: Uint128 = relevant_votes.iter().map(|(_, amount)| amount).sum();

  let mut save_distribution = relevant_votes
    .into_iter()
    .map(|(asset, vp)| {
      Ok(AssetDistribution {
        asset,
        total_vp: vp,
        distribution: Decimal::from_ratio(vp, sum_relevant),
      })
    })
    .collect::<StdResult<Vec<_>>>()?;

  let total: Decimal = save_distribution.iter().map(|a| a.distribution).sum();

  if total > Decimal::percent(100) {
    let remove = total - Decimal::percent(100);
    save_distribution[0].distribution -= remove;
  } else {
    let add = Decimal::percent(100) - total;
    save_distribution[0].distribution += add;
  }

  GAUGE_DISTRIBUTION.save(
    deps.storage,
    (&gauge, period),
    &GaugeDistributionPeriod {
      assets: save_distribution.clone(),
    },
  )?;

  Ok(save_distribution)
}

/// Manages contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
  let contract_version = get_contract_version(deps.storage)?;
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  if contract_version.contract != CONTRACT_NAME {
    return Err(
      StdError::generic_err(format!(
        "contract_name does not match: prev: {0}, new: {1}",
        contract_version.contract, CONTRACT_VERSION
      ))
      .into(),
    );
  }

  Ok(
    Response::new()
      .add_attribute("previous_contract_name", &contract_version.contract)
      .add_attribute("previous_contract_version", &contract_version.version)
      .add_attribute("new_contract_name", CONTRACT_NAME)
      .add_attribute("new_contract_version", CONTRACT_VERSION),
  )
}
