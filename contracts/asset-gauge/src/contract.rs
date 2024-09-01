use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::period_index::Data;
use crate::state::{
  fetch_last_gauge_distribution, fetch_last_gauge_vote, user_idx, AssetIndex,
  GaugeDistributionPeriod, Rebase, UserVotes, CONFIG, GAUGE_DISTRIBUTION, GAUGE_VOTE, LOCK_INFO,
  REBASE, UNCLAIMED_REBASE, USER_ASSET_REWARD_INDEX,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  attr, from_json, Addr, Decimal, Decimal256, DepsMut, Env, MessageInfo, Response, StdResult,
  Storage, Uint128, Uint256,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};
use itertools::Itertools;
use std::collections::HashSet;
use std::convert::TryInto;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::AT_VOTING_ESCROW;
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::helpers::bps::BasicPoints;
use ve3_shared::helpers::governance::get_period;
use ve3_shared::msgs_asset_gauge::{Config, ExecuteMsg, GaugeConfig, InstantiateMsg, ReceiveMsg};
use ve3_shared::msgs_asset_staking::AssetDistribution;
use ve3_shared::msgs_voting_escrow::{End, LockInfoResponse};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let rebase_asset = msg.rebase_asset.check(deps.api, None)?;

  CONFIG.save(
    deps.storage,
    &Config {
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
      gauges: msg.gauges,
      rebase_asset,
    },
  )?;

  REBASE.save(
    deps.storage,
    &Rebase {
      total_fixed: Uint128::zero(),
      global_reward_index: Decimal256::zero(),
    },
  )?;

  Ok(Response::new())
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

    ExecuteMsg::AddRebase {} => {
      let config = CONFIG.load(deps.storage)?;
      let asset = config.rebase_asset.assert_received(&info)?;
      add_rebase(deps, asset)
    },
    ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),

    ExecuteMsg::ClaimRebase {
      token_id,
    } => claim_rebase(deps, env, info.sender, token_id),

    ExecuteMsg::SetDistribution {} => set_distribution(deps, env),

    ExecuteMsg::UpdateConfig {
      update_gauge,
      remove_gauge,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(gauge) = update_gauge {
        if gauge.min_gauge_percentage > Decimal::percent(20) {
          Err(SharedError::NotSupported(
            "min_gauge_percentage needs to be less than 20%".to_string(),
          ))?
        }
        config.gauges.retain(|a| a.name != gauge.name);
        config.gauges.push(gauge);
      }

      if let Some(name) = remove_gauge {
        config.gauges.retain(|a| a.name != name);
      }

      CONFIG.save(deps.storage, &config)?;
      Ok(Response::new().add_attribute("action", "gauge/update_config"))
    },
  }
}

fn receive(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
  let received = Asset::cw20(info.sender, cw20_msg.amount);

  match from_json(&cw20_msg.msg)? {
    ReceiveMsg::AddRebase {} => {
      let config = CONFIG.load(deps.storage)?;
      if received.info != config.rebase_asset {
        return Err(ContractError::InvalidAsset("unsupported rebase cw20".to_string()));
      }
      add_rebase(deps, received)
    },
  }
}

fn claim_rebase(
  deps: DepsMut,
  env: Env,
  user: Addr,
  token_id: Option<String>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let rebase = REBASE.load(deps.storage)?;
  let voting_escrow = config.get_voting_escrow(&deps)?;
  let block_period = get_period(env.block.time.seconds())?;
  let fixed_amount = user_idx().get_latest_fixed(deps.storage, block_period + 1, user.as_str())?;

  calc_rebase_share(deps.storage, &rebase, &user, fixed_amount)?;
  let rebase_amount = UNCLAIMED_REBASE.load(deps.storage, user.clone()).unwrap_or(Uint128::zero());
  UNCLAIMED_REBASE.remove(deps.storage, user.clone());

  if rebase_amount.is_zero() {
    Err(SharedError::InsufficientBalance("no rebase amount".to_string()))?;
  }

  let rebase_asset = config.rebase_asset.with_balance(rebase_amount);

  let msg = match token_id {
    Some(id) => {
      // if id provided -> check if permanent lock
      // if yes, add it to the permanent lock
      let lock = LOCK_INFO.load(deps.storage, &id).map_err(|_| ContractError::LockNotFound)?;
      if lock.end == End::Permanent {
        if lock.asset.info != rebase_asset.info {
          Err(ContractError::RebaseWrongTargetLockAsset)?;
        }

        voting_escrow.create_extend_lock_amount_msg(rebase_asset, id)?
      } else {
        Err(ContractError::RebaseClaimingOnlyForPermanent)?
      }
    },
    // otherwise create a new permanent lock
    None => voting_escrow.create_permanent_lock_msg(rebase_asset, Some(user.to_string()))?,
  };

  let mut resp = Response::new();

  resp = resp
    .add_attribute("action", "gauge/claim_rebase")
    .add_attribute("user", user.as_ref())
    .add_attribute("rebase_amount", rebase_amount.to_string())
    .add_message(msg);

  Ok(resp)
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
  config.assert_gauge(gauge)?;

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

  let allowed =
    config.get_asset_staking(&deps, gauge)?.query_whitelisted_assets_str(&deps.querier)?;

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
        return Err(ContractError::InvalidAsset(addr));
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
    asset_index.change_weights(
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
    (gauge, sender.as_str(), block_period + 1),
    &UserVotes {
      votes,
    },
  )?;

  Ok(
    Response::new()
      .add_attribute("action", "gauge/vote")
      .add_attribute("vp", current_user.total_vp()?),
  )
}

fn remove_votes_of_user(
  storage: &mut dyn Storage,
  config: &Config,
  block_period: u64,
  old_lock: &LockInfoResponse,
) -> Result<Data, ContractError> {
  let user = old_lock.owner.as_str();
  let user_data =
    user_idx().remove_line(storage, block_period + 1, user, BasicPoints::max(), old_lock.into())?;

  // Cancel changes applied by previous votes
  for gauge_config in config.gauges.iter() {
    let gauge = &gauge_config.name;
    let vote = fetch_last_gauge_vote(storage, gauge, user, block_period + 1)?;

    if let Some((_, votes)) = vote {
      let asset_index = AssetIndex::new(gauge);
      let asset_index = asset_index.idx();

      for (key, bps) in votes.votes {
        asset_index.remove_line(storage, block_period + 1, &key, bps, old_lock.into())?;
      }
    }
  }

  Ok(user_data)
}

fn apply_votes_of_user(
  storage: &mut dyn Storage,
  config: &Config,
  block_period: u64,
  new_lock: &LockInfoResponse,
) -> Result<Data, ContractError> {
  let user = new_lock.owner.as_str();
  let user_data =
    user_idx().add_line(storage, block_period + 1, user, BasicPoints::max(), (new_lock).into())?;

  // Cancel changes applied by previous votes
  for gauge_config in config.gauges.iter() {
    let gauge = &gauge_config.name;
    let vote = fetch_last_gauge_vote(storage, gauge, user, block_period + 1)?;

    if let Some((_, votes)) = vote {
      let asset_index = AssetIndex::new(gauge);
      let asset_index = asset_index.idx();

      for (key, bps) in votes.votes {
        asset_index.add_line(storage, block_period + 1, &key, bps, (new_lock).into())?;
      }
    }
  }

  Ok(user_data)
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

  let mut rebase = REBASE.load(deps.storage)?;

  // println!("update vote {token_id}");
  // println!("------- old {old_lock:?}");
  // println!("------- new {new_lock:?}");

  let is_same_owner = old_lock.as_ref().map_or(false, |a| a.owner == new_lock.owner);

  if let Some(old_lock) = old_lock {
    if old_lock.has_vp() {
      let user = remove_votes_of_user(deps.storage, &config, block_period, &old_lock)?;

      rebase.total_fixed = rebase.total_fixed.checked_sub(old_lock.fixed_amount)?;

      if !is_same_owner || !new_lock.has_vp() {
        calc_rebase_share(
          deps.storage,
          &rebase,
          &old_lock.owner,
          user.fixed_amount + old_lock.fixed_amount,
        )?;
      }
    }
  }

  if new_lock.has_vp() {
    let user = apply_votes_of_user(deps.storage, &config, block_period, &new_lock)?;

    rebase.total_fixed = rebase.total_fixed.checked_add(new_lock.fixed_amount)?;
    calc_rebase_share(
      deps.storage,
      &rebase,
      &new_lock.owner,
      user.fixed_amount - new_lock.fixed_amount,
    )?;
  }

  REBASE.save(deps.storage, &rebase)?;

  Ok(Response::new().add_attribute("action", "gauge/update_vote"))
}

fn add_rebase(deps: DepsMut, asset: Asset) -> Result<Response, ContractError> {
  let rebase_distributed = Decimal256::from_ratio(asset.amount, 1u8);

  let mut rebase = REBASE.load(deps.storage)?;

  let total_fixed = rebase.total_fixed;

  if !total_fixed.is_zero() {
    let rate_to_update = rebase_distributed / Decimal256::from_ratio(total_fixed, 1u8);
    if rate_to_update > Decimal256::zero() {
      rebase.global_reward_index += rate_to_update;
      REBASE.save(deps.storage, &rebase)?;
    }
  }

  Ok(
    Response::default()
      .add_attribute("action", "gauge/add_rebase")
      .add_attribute("rebase", asset.to_string()),
  )
}

fn calc_rebase_share(
  storage: &mut dyn Storage,
  rebase: &Rebase,
  user: &Addr,
  balance: Uint128,
) -> Result<Uint128, ContractError> {
  let user_reward_index = USER_ASSET_REWARD_INDEX.load(storage, user.clone());
  let global_reward_index = rebase.global_reward_index;

  if let Ok(user_reward_rate) = user_reward_index {
    let user_staked = balance;
    let user_amount = Uint256::from(user_staked);
    let rewards: Uint128 = ((global_reward_index - user_reward_rate) * user_amount).try_into()?;

    if rewards.is_zero() {
      Ok(Uint128::zero())
    } else {
      USER_ASSET_REWARD_INDEX.save(storage, user.clone(), &global_reward_index)?;
      UNCLAIMED_REBASE.update(storage, user.clone(), |balance| -> Result<_, ContractError> {
        Ok(balance.unwrap_or(Uint128::zero()) + rewards)
      })?;

      Ok(rewards)
    }
  } else {
    // If cannot find user_reward_rate, assume this is the first time they are staking and set it to the current asset_reward_rate
    USER_ASSET_REWARD_INDEX.save(storage, user.clone(), &global_reward_index)?;

    Ok(Uint128::zero())
  }
}

fn set_distribution(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let block_period = get_period(env.block.time.seconds())?;

  let mut attrs = vec![];
  let mut msgs = vec![];
  for gauge_config in config.gauges.iter() {
    let gauge = &gauge_config.name;
    let asset_staking = config.get_asset_staking(&deps, gauge)?;
    let assets = asset_staking.query_whitelisted_assets(&deps.branch().querier)?;
    let mut periods = vec![];

    let distribution =
      match fetch_last_gauge_distribution(deps.branch().storage, gauge, block_period)? {
        Some((last_period, _)) if last_period == block_period => None,

        Some((mut last_period, _)) => {
          while last_period < block_period {
            _set_distribution(deps.branch(), &env, gauge_config, &assets, last_period)?;
            periods.push(last_period);
            last_period += 1;
          }

          periods.push(block_period);
          Some(_set_distribution(deps.branch(), &env, gauge_config, &assets, block_period)?)
        },

        None => {
          periods.push(block_period);
          Some(_set_distribution(deps.branch(), &env, gauge_config, &assets, block_period)?)
        },
      };

    attrs.push(attr("gauge", gauge_config.name.clone()));
    if !periods.is_empty() {
      attrs.push(attr("periods", periods.iter().join(",")));
    }

    if let Some(new_distribution) = distribution {
      // only write if it has assets
      if !new_distribution.is_empty() {
        msgs.push(asset_staking.set_reward_distribution_msg(new_distribution)?)
      }
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

  let total_gauge_vp: Uint128 = allowed_votes.iter().map(|(_, b)| b).sum();
  let min_voting_power = gauge_config.min_gauge_percentage * total_gauge_vp;

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

  if !save_distribution.is_empty() {
    if total > Decimal::percent(100) {
      let remove = total - Decimal::percent(100);
      save_distribution[0].distribution -= remove;
    } else {
      let add = Decimal::percent(100) - total;
      save_distribution[0].distribution += add;
    }
  }

  GAUGE_DISTRIBUTION.save(
    deps.storage,
    (gauge, period),
    &GaugeDistributionPeriod {
      total_gauge_vp,
      assets: save_distribution.clone(),
    },
  )?;

  Ok(save_distribution)
}
