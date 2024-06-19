use crate::constants::{CONTRACT_NAME, CONTRACT_TOTAL_VP_TOKEN_ID, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::operation::Operation;
use crate::query::get_token_lock_info;
use crate::state::{Lock, Point, BLACKLIST, CONFIG, HISTORY, LAST_SLOPE_CHANGE, LOCKED, TOKEN_ID};
use crate::utils::{
  assert_asset_allowed, assert_not_blacklisted, assert_not_blacklisted_all,
  assert_not_decommissioned, assert_periods_remaining, assert_time_limits, calc_voting_power,
  cancel_scheduled_slope, fetch_last_checkpoint, fetch_slope_changes, message_info,
  schedule_slope_change, validate_received_cw20, validate_received_funds,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  attr, from_json, to_json_binary, Addr, Attribute, Binary, CosmosMsg, Deps, DepsMut, Env, Event,
  MessageInfo, Response, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::Asset;
use std::collections::HashSet;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::{
  AT_VE_GUARDIAN, EPOCH_START, MAX_LOCK_PERIODS, MIN_LOCK_PERIODS, SECONDS_PER_WEEK,
};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::decimal_ext::DecimalExt;
use ve3_shared::helpers::general::{addr_opt_fallback, validate_addresses};
use ve3_shared::helpers::governance::{get_period, get_periods_count};
use ve3_shared::helpers::slope::{adjust_vp_and_slope, calc_coefficient};
use ve3_shared::msgs_asset_gauge;
use ve3_shared::msgs_voting_escrow::{
  AssetInfoConfig, Config, DepositAsset, End, ExecuteMsg, InstantiateMsg, LockInfoResponse,
  ReceiveMsg, VeNftCollection, VeNftInfo,
};

/// Creates a new contract with the specified parameters in [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let deposit_assets = validate_deposit_assets(&deps, msg.deposit_assets)?;

  let config = Config {
    global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    push_update_contracts: vec![],
    decommissioned: None,
    deposit_assets,
  };
  CONFIG.save(deps.storage, &config)?;

  let block_period = get_period(env.block.time.seconds())?;
  let point = Point {
    power: Uint128::zero(),
    start: block_period,
    end: End::Period(0),
    slope: Default::default(),
    fixed: Uint128::zero(),
  };
  // Token_id 0 = Total VP
  HISTORY.save(deps.storage, (CONTRACT_TOTAL_VP_TOKEN_ID, block_period), &point)?;
  BLACKLIST.save(deps.storage, &vec![])?;
  TOKEN_ID.save(deps.storage, &Uint128::one())?;

  let nft = VeNftCollection::default();
  nft.instantiate(
    deps,
    env.clone(),
    info,
    cw721_base::InstantiateMsg {
      name: "Vote Escrowed LUNA".to_string(),
      symbol: "veLUNA".to_string(),
      minter: env.contract.address.to_string(),
    },
  )?;

  Ok(Response::default())
}

fn validate_deposit_assets(
  deps: &DepsMut,
  assets: Vec<DepositAsset<String>>,
) -> Result<Vec<DepositAsset<Addr>>, ContractError> {
  assets
    .into_iter()
    .map(|asset| -> Result<_, ContractError> {
      Ok(DepositAsset {
        config: asset.config,
        info: asset.info.check(deps.api, None)?,
      })
    })
    .collect()
}

/// Exposes all the execute functions available in the contract.
///
/// ## Execute messages
/// * **ExecuteMsg::ExtendLockTime { time }** Increase a staker's lock time.
///
/// * **ExecuteMsg::Receive(msg)** Parse incoming messages coming from the ampLP token contract.
///
/// * **ExecuteMsg::Withdraw {}** Withdraw all ampLP from a lock position if the lock has expired.
///
/// * **ExecuteMsg::ProposeNewOwner { owner, expires_in }** Creates a new request to change contract ownership.
///
/// * **ExecuteMsg::DropOwnershipProposal {}** Removes a request to change contract ownership.
///
/// * **ExecuteMsg::ClaimOwnership {}** Claims contract ownership.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  let nft = VeNftCollection::default();

  match msg {
    // OWNER
    ExecuteMsg::UpdateBlacklist {
      append_addrs,
      remove_addrs,
    } => update_blacklist(deps, env, nft, info.sender, append_addrs, remove_addrs),
    ExecuteMsg::UpdateConfig {
      push_update_contracts,
      decommissioned,
      append_deposit_assets,
    } => execute_update_config(
      deps,
      info,
      push_update_contracts,
      decommissioned,
      append_deposit_assets,
    ),

    // USER
    ExecuteMsg::Withdraw {
      token_id,
    } => withdraw(deps, env, nft, info.sender, token_id),
    ExecuteMsg::CreateLock {
      time,
      recipient,
    } => {
      let config = CONFIG.load(deps.storage)?;
      let asset = validate_received_funds(&info.funds, &config.deposit_assets)?;
      let recipient = addr_opt_fallback(deps.api, &recipient, info.sender)?;
      create_lock(deps, env, nft, config, recipient, asset, time)
    },
    ExecuteMsg::ExtendLockTime {
      time,
      token_id,
    } => extend_lock_time(deps, env, nft, info.sender, token_id, time),
    ExecuteMsg::ExtendLockAmount {
      token_id,
    } => {
      let config = CONFIG.load(deps.storage)?;
      let asset = validate_received_funds(&info.funds, &config.deposit_assets)?;
      deposit_for(deps, env, nft, config, info.sender, asset, token_id)
    },
    ExecuteMsg::LockPermanent {
      token_id,
    } => lock_permanent(deps, env, nft, info.sender, token_id),
    ExecuteMsg::UnlockPermanent {
      token_id,
    } => unlock_permanent(deps, env, nft, info.sender, token_id),

    ExecuteMsg::MergeLock {
      token_id,
      token_id_add,
    } => merge_lock(deps, env, nft, info.sender, token_id, token_id_add),
    ExecuteMsg::SplitLock {
      token_id,
      amount,
      recipient,
    } => {
      let recipient = addr_opt_fallback(deps.api, &recipient, info.sender.clone())?;
      split_lock(deps, env, nft, info.sender, token_id, amount, recipient)
    },

    ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),

    ExecuteMsg::TransferNft {
      recipient,
      token_id,
    } => {
      let recipient = deps.api.addr_validate(&recipient)?;
      change_lock_owner(deps, env, nft, info, recipient, token_id, None)
    },
    ExecuteMsg::SendNft {
      contract,
      token_id,
      msg,
    } => {
      let recipient = deps.api.addr_validate(&contract)?;
      change_lock_owner(deps, env, nft, info, recipient, token_id, Some(msg))
    },

    // same as withdraw
    ExecuteMsg::Burn {
      token_id,
    } => withdraw(deps, env, nft, info.sender, token_id),

    // Approve, Revoke, ApproveAll, RevokeAll
    _ => Ok(nft.execute(deps, env, info, msg.into())?),
  }
}

fn receive(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
  let received = Asset::cw20(info.sender, cw20_msg.amount);
  let sender = deps.api.addr_validate(&cw20_msg.sender)?;
  let nft = VeNftCollection::default();

  match from_json(&cw20_msg.msg)? {
    ReceiveMsg::CreateLock {
      time,
      recipient,
    } => {
      let config = CONFIG.load(deps.storage)?;
      let asset_validated = validate_received_cw20(received, &config.deposit_assets)?;
      let recipient = addr_opt_fallback(deps.api, &recipient, sender)?;
      create_lock(deps, env, nft, config, recipient, asset_validated, time)
    },
    ReceiveMsg::ExtendLockAmount {
      token_id,
    } => {
      let config = CONFIG.load(deps.storage)?;
      let asset = validate_received_cw20(received, &config.deposit_assets)?;
      deposit_for(deps, env, nft, config, sender, asset, token_id)
    },
  }
}

/// Checkpoint the total voting power (total supply of vAMP).
/// This function fetches the last available vAMP checkpoint, recalculates passed periods since the checkpoint and until now,
/// applies slope changes and saves all recalculated periods in [`HISTORY`].
///
/// * **add_voting_power** amount of vAMP to add to the total.
///
/// * **reduce_power** amount of vAMP to subtract from the total.
///
/// * **old_slope** old slope applied to the total voting power (vAMP supply).
///
/// * **new_slope** new slope to be applied to the total voting power (vAMP supply).
#[allow(clippy::too_many_arguments)]
fn checkpoint_total(
  storage: &mut dyn Storage,
  env: Env,
  voting_power_change: Operation,
  fixed_change: Operation,
  old_slope: Uint128,
  new_slope: Uint128,
) -> Result<(), ContractError> {
  let block_period = get_period(env.block.time.seconds())?;

  // Get last checkpoint
  let last_checkpoint = fetch_last_checkpoint(storage, CONTRACT_TOTAL_VP_TOKEN_ID, block_period)?;
  let new_point = if let Some((_, mut point)) = last_checkpoint {
    let last_slope_change = LAST_SLOPE_CHANGE.may_load(storage)?.unwrap_or(0);
    if last_slope_change < block_period {
      let scheduled_slope_changes = fetch_slope_changes(storage, last_slope_change, block_period)?;
      // Recalculating passed points
      for (recalc_period, scheduled_change) in scheduled_slope_changes {
        point = Point {
          power: calc_voting_power(&point, recalc_period),
          start: recalc_period,
          slope: point.slope.saturating_sub(scheduled_change),
          ..point
        };
        HISTORY.save(storage, (CONTRACT_TOTAL_VP_TOKEN_ID, recalc_period), &point)?
      }

      LAST_SLOPE_CHANGE.save(storage, &block_period)?
    }

    let current_vp = calc_voting_power(&point, block_period);
    let new_power = voting_power_change.apply_to(current_vp)?;

    Point {
      power: new_power,
      slope: point.slope.saturating_sub(old_slope) + new_slope,
      start: block_period,
      fixed: fixed_change.apply_to(point.fixed)?,
      end: point.end,
    }
  } else {
    Point {
      power: voting_power_change.add_amount().unwrap_or_default(),
      slope: new_slope,
      start: block_period,
      // we don't use 'end' in total voting power calculations
      end: End::Period(0),
      fixed: fixed_change.add_amount().unwrap_or_default(),
    }
  };
  HISTORY.save(storage, (CONTRACT_TOTAL_VP_TOKEN_ID, block_period), &new_point)?;
  Ok(())
}

/// Checkpoint a user's voting power (vAMP balance).
/// This function fetches the user's last available checkpoint, calculates the user's current voting power, applies slope changes based on
/// `add_amount` and `new_end` parameters, schedules slope changes for total voting power and saves the new checkpoint for the current
/// period in [`HISTORY`] (using the user's address).
/// If a user already checkpointed themselves for the current period, then this function uses the current checkpoint as the latest
/// available one.
///
/// * **addr** staker for which we checkpoint the voting power.
///
/// * **add_amount** amount of vAMP to add to the staker's balance.
///
/// * **new_end** new lock time for the staker's vAMP position.
fn checkpoint(
  store: &mut dyn Storage,
  env: Env,
  token_id: &str,
  underlying_change: Operation,
  new_end: Option<End>,
) -> Result<(), ContractError> {
  let block_period = get_period(env.block.time.seconds())?;
  let mut old_slope = Default::default();
  let mut voting_power_change = Operation::None;

  // Get the last user checkpoint
  let last_checkpoint = fetch_last_checkpoint(store, token_id, block_period)?;
  let new_point = if let Some((_, point)) = last_checkpoint {
    let end = new_end.unwrap_or(point.end.clone());
    let current_power = calc_voting_power(&point, block_period);

    // Cancel the previously scheduled slope change (same logic as in cancel_scheduled_slope)
    if let Some((last_slope_change, end)) = cancel_scheduled_slope(store, point.slope, &point.end)?
    {
      if end > last_slope_change {
        // We need to subtract the slope point from the total voting power slope
        // Only if the point is still active and has not been processed/applied yet.
        old_slope = point.slope
      };
    }

    match end {
      End::Period(new_end) => {
        let dt = new_end.saturating_sub(block_period);

        let new_slope = if dt != 0 {
          // always recalculate slope when the end has changed
          let recalculate = match &point.end {
            End::Permanent => true,
            End::Period(existing_end) => new_end > *existing_end,
          };

          if recalculate {
            // This is extend_lock_time. Recalculating user's voting power
            let mut lock = LOCKED.load(store, token_id)?;
            let mut new_voting_power =
              calc_coefficient(dt).checked_mul_uint(lock.underlying_amount)?;
            let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?; // end_vp

            voting_power_change = Operation::from_values(new_voting_power, current_power);

            lock.last_extend_lock_period = block_period;
            LOCKED.save(store, token_id, &lock, env.block.height)?;
            slope
          } else {
            // This is an increase in the user's lock amount
            let mut new_voting_power = match underlying_change {
              Operation::None => current_power,
              Operation::Add(add_amount) => {
                let raw_add_voting_power = calc_coefficient(dt).checked_mul_uint(add_amount)?;
                current_power.checked_add(raw_add_voting_power)?
              },
              Operation::Reduce(reduce_amount) => {
                let raw_reduce_voting_power =
                  calc_coefficient(dt).checked_mul_uint(reduce_amount)?;
                current_power.saturating_sub(raw_reduce_voting_power)
              },
            };

            let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?;

            voting_power_change = Operation::from_values(new_voting_power, current_power);

            slope
          }
        } else {
          // current period > new end
          Uint128::zero()
        };

        Point {
          power: voting_power_change.apply_to(current_power)?,
          slope: new_slope,
          start: block_period,
          end: End::Period(new_end),
          fixed: underlying_change.apply_to(point.fixed)?,
        }
      },
      End::Permanent => {
        // new end permanent

        let fixed = underlying_change.apply_to(point.fixed)?;
        let new_voting_power = calc_coefficient(MAX_LOCK_PERIODS).checked_mul_uint(fixed)?;
        voting_power_change = Operation::from_values(new_voting_power, current_power);

        Point {
          power: new_voting_power,
          slope: Uint128::zero(),
          start: block_period,
          end,
          fixed,
        }
      },
    }
  } else {
    // no existing checkpoint found

    // This error can't happen since this if-branch is intended for checkpoint creation
    let end = new_end.ok_or(ContractError::CheckpointInitializationFailed {})?;
    let add_amount = underlying_change
      .add_amount()
      .ok_or(SharedError::NotSupported("requires an amount for point creation".to_string()))?;

    let (add_voting_power, slope) = match end {
      End::Permanent => {
        let add_voting_power = calc_coefficient(MAX_LOCK_PERIODS).checked_mul_uint(add_amount)?;
        (add_voting_power, Uint128::zero())
      },
      End::Period(end) => {
        let dt = end - block_period;
        let mut add_voting_power = calc_coefficient(dt).checked_mul_uint(add_amount)?;
        let slope = adjust_vp_and_slope(&mut add_voting_power, dt)?;
        (add_voting_power, slope)
      },
    };

    voting_power_change = Operation::Add(add_voting_power);
    Point {
      power: add_voting_power,
      slope,
      start: block_period,
      end,
      fixed: add_amount,
    }
  };

  // Schedule a slope change
  schedule_slope_change(store, new_point.slope, &new_point.end)?;

  HISTORY.save(store, (token_id, block_period), &new_point)?;

  checkpoint_total(store, env, voting_power_change, underlying_change, old_slope, new_point.slope)
}

/// Creates a lock for the user that lasts for the specified time duration (in seconds).
/// Checks that the user is locking ampLP tokens.
/// Checks that the lock time is within [`WEEK`]..[`MAX_LOCK_TIME`].
/// Creates a lock if it doesn't exist and triggers a [`checkpoint`] for the staker.
/// If a lock already exists, then a [`ContractError`] is returned.
///
/// * **user** staker for which we create a lock position.
///
/// * **amount** amount of ampLP deposited in the lock position.
///
/// * **time** duration of the lock.
fn create_lock(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  config: Config,
  sender: Addr,
  asset: Asset,
  time: Option<u64>,
) -> Result<Response, ContractError> {
  assert_not_decommissioned(&config)?;
  assert_not_blacklisted(deps.storage, &sender)?;
  assert_time_limits(time)?;

  let asset_config = assert_asset_allowed(&config, &asset)?;
  let underlying_amount = asset_config.get_underlying_amount(&deps.querier, asset.amount)?;

  // end time only when it has a time
  // no time means permanent lock
  let end = if let Some(time) = time {
    let block_period = get_period(env.block.time.seconds())?;
    let periods = get_periods_count(time);
    End::Period(block_period + periods)
  } else {
    End::Permanent
  };

  _create_lock(deps, env, nft, &config, asset, underlying_amount, sender, end)
}

#[allow(clippy::too_many_arguments)]
fn _create_lock(
  mut deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  config: &Config,
  asset: Asset,
  underlying_amount: Uint128,
  recipient: Addr,
  end: End,
) -> Result<Response, ContractError> {
  let token_id_uint = TOKEN_ID.load(deps.storage)?;
  TOKEN_ID.save(deps.storage, &token_id_uint.checked_add(Uint128::one())?)?;
  let token_id = token_id_uint.to_string();

  let block_period = get_period(env.block.time.seconds())?;
  let start = block_period;
  assert_periods_remaining(&end, start)?;

  let lock = Lock {
    asset,
    underlying_amount,
    start,
    end: end.clone(),
    last_extend_lock_period: block_period,
    owner: recipient.clone(),
  };

  // save lock & create NFT
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;
  let mint_response = nft.mint(
    deps.branch(),
    message_info(env.contract.address.clone()),
    token_id.to_string(),
    recipient.to_string(),
    None,
    lock.get_nft_extension(),
  )?;

  checkpoint(deps.storage, env.clone(), &token_id, Operation::Add(underlying_amount), Some(end))?;

  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id))
      .add_attribute("action", "ve/create_lock")
      .add_attribute("voting_power", lock_info.voting_power.to_string())
      .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock_info.end_string())
      .add_attributes(mint_response.attributes)
      .add_messages(get_push_update_msgs(config, &token_id, Ok(lock_info))?),
  )
}

fn merge_lock(
  mut deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id_1: String,
  token_id_2: String,
) -> Result<Response, ContractError> {
  let (config, mut lock1, mut token1, asset_config) =
    _get_lock_context(&deps, &sender, &token_id_1, &nft, &env, None, true)?;

  let lock2 = LOCKED
    .load(deps.storage, &token_id_2)
    .map_err(|_| ContractError::LockDoesNotExist(token_id_2.to_string()))?;

  if lock1.asset.info != lock2.asset.info {
    return Err(ContractError::LocksNeedSameAssets(token_id_1.to_string(), token_id_2.to_string()));
  }

  if lock1.end != lock2.end {
    return Err(ContractError::LocksNeedSameEnd(token_id_1.to_string(), token_id_2.to_string()));
  }

  // update existing lock that is reduced by new_lock_amount
  lock1.asset.amount = lock1.asset.amount.checked_add(lock2.asset.amount)?;
  let underlying_change = lock1.update_underlying(&deps, &asset_config)?;

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id_1, &lock1, env.block.height)?;
  token1.extension = lock1.get_nft_extension();
  nft.tokens.save(deps.storage, &token_id_1, &token1)?;

  checkpoint(deps.storage, env.clone(), &token_id_1, underlying_change, None)?;

  // burn lock 2 without transfering assets.
  // this also removes it from LOCKED
  // burn checks that the sender has approval for the lock or is the owner
  let block_period = get_period(env.block.time.seconds())?;
  let burn_attrs = _burn(&mut deps, &env, nft, sender, &token_id_2, lock2, block_period)?;

  let lock1_info = get_token_lock_info(deps.as_ref(), &env, &token_id_1, None)?;
  let lock2_info = get_token_lock_info(deps.as_ref(), &env, &token_id_2, None)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id_1))
      .add_event(get_metadata_changed(&token_id_2))
      .add_attribute("action", "ve/merge_lock")
      .add_attribute("merge", format!("{0},{1}", token_id_1, token_id_2))
      .add_attribute("voting_power", lock1_info.voting_power.to_string())
      .add_attribute("fixed_power", lock1_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock1_info.end_string())
      .add_messages(get_push_update_msgs(&config, &token_id_1, Ok(lock1_info))?)
      // add burnt lock attrs
      .add_attributes(burn_attrs)
      .add_messages(get_push_update_msgs(&config, &token_id_2, Ok(lock2_info))?),
  )
}

fn split_lock(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id: String,
  new_lock_amount: Uint128,
  recipient: Addr,
) -> Result<Response, ContractError> {
  let (config, mut lock, mut token, asset_config) =
    _get_lock_context(&deps, &sender, &token_id, &nft, &env, None, true)?;

  let block_period = get_period(env.block.time.seconds())?;
  let start = block_period;
  assert_periods_remaining(&lock.end, start)?;

  // update existing lock that is reduced by new_lock_amount
  let exchange_rate = asset_config.get_exchange_rate(&deps.querier)?;
  lock.asset.amount = lock
    .asset
    .amount
    .checked_sub(new_lock_amount)
    .map_err(|_| ContractError::LockNotEnoughFunds {})?;

  let new_underlying_value = exchange_rate.map_or(lock.asset.amount, |e| e * lock.asset.amount);
  let underlying_change = lock.update_underlying_value(new_underlying_value)?;

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;
  token.extension = lock.get_nft_extension();
  nft.tokens.save(deps.storage, &token_id, &token)?;

  checkpoint(deps.storage, env.clone(), &token_id, underlying_change, None)?;
  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  // creating new lock
  let new_asset = lock.asset.info.with_balance(new_lock_amount);
  let new_underlying = exchange_rate.map_or(lock.asset.amount, |e| e * new_lock_amount);
  let create_response =
    _create_lock(deps, env, nft, &config, new_asset, new_underlying, recipient, lock.end)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id))
      .add_attribute("action", "ve/split_lock")
      .add_attribute("voting_power", lock_info.voting_power.to_string())
      .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock_info.end_string())
      .add_messages(get_push_update_msgs(&config, &token_id, Ok(lock_info))?)
      // add new lock msgs
      .add_attributes(create_response.attributes)
      .add_submessages(create_response.messages),
  )
}

fn get_metadata_changed(token_id: &str) -> Event {
  Event::new("metadata_changed").add_attribute("token_id", token_id.to_string())
}

/// Deposits an 'amount' of ampLP tokens into 'user''s lock.
/// Checks that the user is transferring and locking ampLP.
/// Triggers a [`checkpoint`] for the user.
/// If the user does not have a lock, then a [`ContractError`] is returned.
///
/// * **amount** amount of ampLP to deposit.
///
/// * **user** user who's lock amount will increase.
fn deposit_for(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  config: Config,
  sender: Addr,
  asset: Asset,
  token_id: String,
) -> Result<Response, ContractError> {
  // only place to not validate ownership
  let (config, mut lock, mut token, asset_config) =
    _get_lock_context(&deps, &sender, &token_id, &nft, &env, Some(config), false)?;

  if lock.asset.info != asset.info {
    return Err(ContractError::WrongAssetExpected(
      asset.info.to_string(),
      lock.asset.info.to_string(),
    ));
  }

  let block_period = get_period(env.block.time.seconds())?;
  let mut new_end = None;

  if let End::Period(end) = &lock.end {
    if *end < block_period + MIN_LOCK_PERIODS {
      // extension of lock is only allowed for owner
      // but generally donations to other locks is okay
      nft.check_can_send(deps.as_ref(), &env, &message_info(sender.clone()), &token)?;

      lock.end = End::Period(block_period + MIN_LOCK_PERIODS);
      new_end = Some(lock.end.clone());
    }
  }

  lock.asset.amount = lock.asset.amount.checked_add(asset.amount)?;

  // recalculating the underlying amount for the whole lock
  let underlying_change = lock.update_underlying(&deps, &asset_config)?;

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;
  token.extension = lock.get_nft_extension();
  nft.tokens.save(deps.storage, &token_id, &token)?;

  checkpoint(deps.storage, env.clone(), &token_id, underlying_change, new_end)?;

  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id))
      .add_attribute("action", "ve/deposit_for")
      .add_attribute("voting_power", lock_info.voting_power.to_string())
      .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock_info.end_string())
      .add_messages(get_push_update_msgs(&config, &token_id, Ok(lock_info))?),
  )
}

fn change_lock_owner(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  info: MessageInfo,
  recipient: Addr,
  token_id: String,
  msg: Option<Binary>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_not_blacklisted_all(deps.storage, vec![info.sender.clone(), recipient.clone()])?;
  let mut lock = LOCKED
    .load(deps.storage, &token_id)
    .map_err(|_| ContractError::LockDoesNotExist(token_id.to_string()))?;

  let old_owner = lock.owner;
  lock.owner = recipient.clone();

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;

  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  let resp = if let Some(msg) = msg {
    nft.execute(
      deps,
      env,
      info,
      cw721_base::ExecuteMsg::SendNft {
        contract: recipient.to_string(),
        token_id: token_id.clone(),
        msg,
      },
    )?
  } else {
    nft.execute(
      deps,
      env,
      info,
      cw721_base::ExecuteMsg::TransferNft {
        recipient: recipient.to_string(),
        token_id: token_id.clone(),
      },
    )?
  };

  Ok(
    Response::new()
      .add_attribute("action", "ve/change_lock_owner")
      .add_attribute("old_owner", old_owner.to_string())
      .add_attribute("new_owner", lock.owner.to_string())
      .add_messages(get_push_update_msgs(&config, &token_id, Ok(lock_info))?)
      .add_attributes(resp.attributes)
      .add_submessages(resp.messages),
  )
}
/// Increase the current lock time for a staker by a specified time period.
/// Evaluates that the `time` is within [`WEEK`]..[`MAX_LOCK_TIME`]
/// and then it triggers a [`checkpoint`].
/// If the user lock doesn't exist or if it expired, then a [`ContractError`] is returned.
///
/// ## Note
/// The time is added to the lock's `end`.
/// For example, at period 0, the user has their ampLP locked for 3 weeks.
/// In 1 week, they increase their lock time by 10 weeks, thus the unlock period becomes 13 weeks.
///
/// * **time** increase in lock time applied to the staker's position.
fn extend_lock_time(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id: String,
  time: u64,
) -> Result<Response, ContractError> {
  let (config, mut lock, mut token, asset_config) =
    _get_lock_context(&deps, &sender, &token_id, &nft, &env, None, true)?;

  // Disable the ability to extend the lock time by less than a week
  assert_time_limits(Some(time))?;

  let block_period = get_period(env.block.time.seconds())?;
  if let End::Period(mut end) = &lock.end {
    if end < block_period {
      // extend_lock_time always starts from the current period.
      end = block_period;
    };

    end += get_periods_count(time);
    lock.end = End::Period(end);

    // Should not exceed MAX_LOCK_TIME
    assert_time_limits(Some(EPOCH_START + end * SECONDS_PER_WEEK - env.block.time.seconds()))?;
  } else {
    return Err(ContractError::LockIsPermanent("cannot extend".to_string()));
  }

  let underlying_change = lock.update_underlying(&deps, &asset_config)?;

  let start = block_period;
  assert_periods_remaining(&lock.end, start)?;

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;
  token.extension = lock.get_nft_extension();
  nft.tokens.save(deps.storage, &token_id, &token)?;

  checkpoint(deps.storage, env.clone(), &token_id, underlying_change, Some(lock.end))?;

  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id))
      .add_attribute("action", "ve/extend_lock_time")
      .add_attribute("voting_power", lock_info.voting_power.to_string())
      .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock_info.end_string())
      .add_messages(get_push_update_msgs(&config, &token_id, Ok(lock_info))?),
  )
}

fn lock_permanent(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id: String,
) -> Result<Response, ContractError> {
  let (config, mut lock, mut token, asset_config) =
    _get_lock_context(&deps, &sender, &token_id, &nft, &env, None, true)?;

  if lock.end == End::Permanent {
    return Err(ContractError::LockIsPermanent("already permanent".to_string()));
  }

  lock.end = End::Permanent;

  let underlying_change = lock.update_underlying(&deps, &asset_config)?;

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;
  token.extension = lock.get_nft_extension();
  nft.tokens.save(deps.storage, &token_id, &token)?;

  checkpoint(deps.storage, env.clone(), &token_id, underlying_change, Some(lock.end))?;

  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id))
      .add_attribute("action", "ve/lock_permanent")
      .add_attribute("voting_power", lock_info.voting_power.to_string())
      .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock_info.end_string())
      .add_messages(get_push_update_msgs(&config, &token_id, Ok(lock_info))?),
  )
}

fn unlock_permanent(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id: String,
) -> Result<Response, ContractError> {
  let (config, mut lock, mut token, asset_config) =
    _get_lock_context(&deps, &sender, &token_id, &nft, &env, None, true)?;

  if lock.end != End::Permanent {
    return Err(ContractError::LockIsNotPermanent);
  }

  let block_period = get_period(env.block.time.seconds())?;
  lock.end = End::Period(block_period + MAX_LOCK_PERIODS);

  let underlying_change = lock.update_underlying(&deps, &asset_config)?;

  // save lock & keep NFT data in sync
  LOCKED.save(deps.storage, &token_id, &lock, env.block.height)?;
  token.extension = lock.get_nft_extension();
  nft.tokens.save(deps.storage, &token_id, &token)?;

  checkpoint(deps.storage, env.clone(), &token_id, underlying_change, Some(lock.end))?;

  let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None)?;

  Ok(
    Response::default()
      .add_event(get_metadata_changed(&token_id))
      .add_attribute("action", "ve/unlock_permanent")
      .add_attribute("voting_power", lock_info.voting_power.to_string())
      .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
      .add_attribute("lock_end", lock_info.end_string())
      .add_messages(get_push_update_msgs(&config, &token_id, Ok(lock_info))?),
  )
}

fn _get_lock_context(
  deps: &DepsMut,
  sender: &Addr,
  token_id: &String,
  nft: &VeNftCollection,
  env: &Env,
  config: Option<Config>,
  validate_ownership: bool,
) -> Result<(Config, Lock, VeNftInfo, AssetInfoConfig), ContractError> {
  let config = if let Some(config) = config {
    config
  } else {
    CONFIG.load(deps.storage)?
  };
  assert_not_blacklisted(deps.storage, sender)?;
  assert_not_decommissioned(&config)?;
  let lock = LOCKED
    .load(deps.storage, token_id)
    .map_err(|_| ContractError::LockDoesNotExist(token_id.to_string()))?;
  let token = nft
    .tokens
    .load(deps.storage, token_id)
    .map_err(|_| ContractError::LockDoesNotExist(token_id.to_string()))?;
  let asset_config = assert_asset_allowed(&config, &lock.asset)?;

  if validate_ownership {
    nft.check_can_send(deps.as_ref(), env, &message_info(sender.clone()), &token)?;
  }
  Ok((config, lock, token, asset_config))
}

/// Withdraws the whole amount of locked ampLP from a specific user lock.
/// If the user lock doesn't exist or if it has not yet expired, then a [`ContractError`] is returned.
fn withdraw(
  mut deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id: String,
) -> Result<Response, ContractError> {
  let lock = LOCKED
    .load(deps.storage, &token_id)
    .map_err(|_| ContractError::LockDoesNotExist(token_id.to_string()))?;

  let block_period = get_period(env.block.time.seconds())?;
  let config = CONFIG.load(deps.storage)?;
  let is_decommissioned = config.decommissioned.unwrap_or_default();

  let attrs;

  let expired = is_decommissioned
    || match lock.end {
      End::Permanent => false,
      End::Period(end) => end <= block_period,
    };

  // println!("lock {block_period} {lock:?}");

  if !expired {
    Err(ContractError::LockHasNotExpired {})
  } else {
    let transfer_msg = lock.asset.transfer_msg(sender.clone())?;

    attrs = _burn(&mut deps, &env, nft, sender, &token_id, lock, block_period)?;

    let lock_info = get_token_lock_info(deps.as_ref(), &env, &token_id, None);
    let msgs = get_push_update_msgs(&config, &token_id, lock_info)?;

    Ok(
      Response::default()
        .add_message(transfer_msg)
        .add_messages(msgs)
        .add_attribute("action", "ve/withdraw")
        .add_attributes(attrs),
    )
  }
}

fn _burn(
  deps: &mut DepsMut,
  env: &Env,
  nft: VeNftCollection,
  sender: Addr,
  token_id: &String,
  mut lock: Lock,
  block_period: u64,
) -> Result<Vec<Attribute>, ContractError> {
  let burn_resp = nft.execute(
    deps.branch(),
    env.clone(),
    message_info(sender.clone()),
    cw721_base::ExecuteMsg::Burn {
      token_id: token_id.to_string(),
    },
  )?;
  let reduce_amount = lock.underlying_amount;
  lock.asset = lock.asset.info.with_balance(Uint128::zero());
  lock.underlying_amount = Uint128::zero();
  LOCKED.save(deps.storage, token_id, &lock, env.block.height)?;

  let recalculate = match lock.end {
    End::Permanent => true,
    End::Period(end) => end > block_period,
  };

  if recalculate {
    // early withdraw through decommissioned or merge.
    // Update voting power same as blacklist.
    let last_checkpoint = fetch_last_checkpoint(deps.storage, token_id, block_period)?;
    if let Some((_, point)) = last_checkpoint {
      // We need to checkpoint with zero power and zero slope
      HISTORY.save(
        deps.storage,
        (token_id, block_period),
        &Point {
          power: Uint128::zero(),
          slope: Default::default(),
          start: block_period,
          end: End::Period(block_period),
          fixed: Uint128::zero(),
        },
      )?;

      let cur_power = calc_voting_power(&point, block_period);

      // User's contribution in the total voting power calculation
      let reduce_total_vp = cur_power;
      let old_slopes = point.slope;
      let old_amount = point.fixed;
      cancel_scheduled_slope(deps.storage, point.slope, &point.end)?;

      checkpoint_total(
        deps.storage,
        env.clone(),
        Operation::Reduce(reduce_total_vp),
        Operation::Reduce(old_amount),
        old_slopes,
        Default::default(),
      )?;
    }
  } else {
    // We need to checkpoint and eliminate the slope influence on a future lock
    HISTORY.save(
      deps.storage,
      (token_id, block_period),
      &Point {
        power: Uint128::zero(),
        start: block_period,
        end: End::Period(block_period),
        slope: Default::default(),
        fixed: Uint128::zero(),
      },
    )?;

    // normal withdraw
    // removing funds needs to remove from total checkpoint aswell.
    checkpoint_total(
      deps.storage,
      env.clone(),
      Operation::None,
      Operation::Reduce(reduce_amount),
      Default::default(),
      Default::default(),
    )?;
  }

  Ok(burn_resp.attributes)
}

fn get_push_update_msgs_multi(
  deps: Deps,
  env: Env,
  config: Config,
  token_ids: Vec<String>,
) -> StdResult<Vec<CosmosMsg>> {
  let results: Vec<CosmosMsg> = token_ids
    .into_iter()
    .map(|token_id| {
      let lock_info = get_token_lock_info(deps, &env, &token_id, None);
      get_push_update_msgs(&config, &token_id, lock_info)
    })
    .collect::<StdResult<Vec<_>>>()?
    .into_iter()
    .flatten()
    .collect();

  Ok(results)
}

fn get_push_update_msgs(
  config: &Config,
  token_id: &str,
  lock_info: Result<LockInfoResponse, ContractError>,
) -> StdResult<Vec<CosmosMsg>> {
  // only send update if lock info is available. LOCK info is never removed for any user that locked anything.
  if let Ok(lock_info) = lock_info {
    config
      .push_update_contracts
      .iter()
      .map(|contract| {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
          contract_addr: contract.to_string(),
          msg: to_json_binary(&msgs_asset_gauge::ExecuteMsg::UpdateVote {
            token_id: token_id.to_string(),
            lock_info: lock_info.clone(),
          })?,
          funds: vec![],
        }))
      })
      .collect::<StdResult<Vec<_>>>()
  } else {
    Ok(vec![])
  }
}

/// Update the staker blacklist. Whitelists addresses specified in 'remove_addrs'
/// and blacklists new addresses specified in 'append_addrs'. Nullifies staker voting power and
/// cancels their contribution in the total voting power (total vAMP supply).
///
/// * **append_addrs** array of addresses to blacklist.
///
/// * **remove_addrs** array of addresses to whitelist.
fn update_blacklist(
  deps: DepsMut,
  env: Env,
  nft: VeNftCollection,
  sender: Addr,
  append_addrs: Option<Vec<String>>,
  remove_addrs: Option<Vec<String>>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  // Permission check
  config.global_config().assert_owner_or_address_type(&deps.querier, AT_VE_GUARDIAN, &sender)?;

  let append_addrs = append_addrs.unwrap_or_default();
  let remove_addrs = remove_addrs.unwrap_or_default();
  let blacklist = BLACKLIST.load(deps.storage)?;
  let append: Vec<_> = validate_addresses(deps.api, &append_addrs)?
    .into_iter()
    .filter(|addr| !blacklist.contains(addr))
    .collect();
  let remove: Vec<_> = validate_addresses(deps.api, &remove_addrs)?
    .into_iter()
    .filter(|addr| blacklist.contains(addr))
    .collect();

  if append.is_empty() && remove.is_empty() {
    return Err(ContractError::AddressBlacklistEmpty {});
  }

  let block_period = get_period(env.block.time.seconds())?;
  let mut reduce_total_vp = Uint128::zero(); // accumulator for decreasing total voting power
  let mut old_slopes = Uint128::zero(); // accumulator for old slopes
  let mut old_amount = Uint128::zero(); // accumulator for old amount

  let mut used_addr: HashSet<Addr> = HashSet::new();
  let mut ids = vec![];

  for addr in append.iter() {
    if !used_addr.insert(addr.clone()) {
      return Err(ContractError::AddressBlacklistDuplicated(addr.to_string()));
    }

    for token_id in nft
      .tokens
      .idx
      .owner
      .prefix(addr.clone())
      .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
      .collect::<Vec<_>>()
    {
      let token_id = token_id?;
      let last_checkpoint = fetch_last_checkpoint(deps.storage, &token_id, block_period)?;
      if let Some((_, point)) = last_checkpoint {
        // We need to checkpoint with zero power and zero slope
        HISTORY.save(
          deps.storage,
          (&token_id, block_period),
          &Point {
            power: Uint128::zero(),
            slope: Default::default(),
            start: block_period,
            end: End::Period(block_period),
            fixed: Uint128::zero(),
          },
        )?;

        let cur_power = calc_voting_power(&point, block_period);
        // User's contribution is already zero. Skipping them
        if cur_power.is_zero() {
          continue;
        }

        // User's contribution in the total voting power calculation
        reduce_total_vp += cur_power;
        old_slopes += point.slope;
        old_amount += point.fixed;
        cancel_scheduled_slope(deps.storage, point.slope, &point.end)?;
      }
      ids.push(token_id);
    }
  }

  if !reduce_total_vp.is_zero() || !old_slopes.is_zero() {
    // Trigger a total voting power recalculation
    checkpoint_total(
      deps.storage,
      env.clone(),
      Operation::Reduce(reduce_total_vp),
      Operation::Reduce(old_amount),
      old_slopes,
      Default::default(),
    )?;
  }

  for addr in remove.iter() {
    if !used_addr.insert(addr.clone()) {
      return Err(ContractError::AddressBlacklistDuplicated(addr.to_string()));
    }

    for token_id in nft
      .tokens
      .idx
      .owner
      .prefix(addr.clone())
      .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
      .collect::<Vec<_>>()
    {
      let token_id = token_id?;
      let lock_opt = LOCKED.may_load(deps.storage, &token_id)?;
      if let Some(Lock {
        underlying_amount,
        end,
        ..
      }) = lock_opt
      {
        checkpoint(
          deps.storage,
          env.clone(),
          &token_id,
          Operation::Add(underlying_amount),
          Some(end),
        )?;
      }
      ids.push(token_id);
    }
  }

  BLACKLIST.update(deps.storage, |blacklist| -> StdResult<Vec<Addr>> {
    let mut updated_blacklist: Vec<_> =
      blacklist.into_iter().filter(|addr| !remove.contains(addr)).collect();
    updated_blacklist.extend(append.clone());
    Ok(updated_blacklist)
  })?;

  let mut attrs = vec![attr("action", "ve/update_blacklist")];
  if !append_addrs.is_empty() {
    attrs.push(attr("added_addresses", append_addrs.join(",")))
  }
  if !remove_addrs.is_empty() {
    attrs.push(attr("removed_addresses", remove_addrs.join(",")))
  }

  Ok(Response::default().add_attributes(attrs).add_messages(get_push_update_msgs_multi(
    deps.as_ref(),
    env.clone(),
    config.clone(),
    ids,
  )?))
}

/// Updates contracts' guardian address.
fn execute_update_config(
  deps: DepsMut,
  info: MessageInfo,
  push_update_contracts: Option<Vec<String>>,
  decommissioned: Option<bool>,
  append_deposit_assets: Option<Vec<DepositAsset<String>>>,
) -> Result<Response, ContractError> {
  let mut config = CONFIG.load(deps.storage)?;

  config.global_config().assert_owner(&deps.querier, &info.sender)?;

  if let Some(decommissioned) = decommissioned {
    if decommissioned {
      config.decommissioned = Some(true);
    }
  }

  if let Some(push_update_contracts) = push_update_contracts {
    config.push_update_contracts = push_update_contracts
      .iter()
      .map(|c| deps.api.addr_validate(c))
      .collect::<StdResult<Vec<_>>>()?;
  }

  if let Some(append_deposit_assets) = append_deposit_assets {
    let deposit_assets = validate_deposit_assets(&deps, append_deposit_assets)?;

    for deposit_asset in deposit_assets.into_iter() {
      match &deposit_asset.config {
        AssetInfoConfig::Default => (),
        AssetInfoConfig::ExchangeRate {
          contract,
        } => {
          deps.api.addr_validate(contract.as_str())?;
        },
      }

      config.deposit_assets.retain(|a| a.info != deposit_asset.info);
      config.deposit_assets.push(deposit_asset);
    }
  }

  CONFIG.save(deps.storage, &config)?;

  Ok(Response::default().add_attribute("action", "ve/execute_update_config"))
}
