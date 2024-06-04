#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr,  from_json, to_json_binary, Addr, , CosmosMsg, Deps, DepsMut, 
    Env, MessageInfo, Response,  StdResult, Storage, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw20_base::state::{MinterData, TokenInfo,  TOKEN_INFO};
use cw_asset::{Asset,  AssetInfoUnchecked};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use ve3_global_config::global_config_adapter::ConfigExt;
use ve3_shared::constants::{EPOCH_START, MIN_LOCK_PERIODS, WEEK};
use ve3_shared::extensions::decimal_ext::DecimalExt;
use ve3_shared::helpers::governance::{get_period, get_periods_count};
use ve3_shared::helpers::slope::{adjust_vp_and_slope, calc_coefficient};
use ve3_shared::voting_escrow::{
    AssetInfoLockConfig, Config, ExecuteMsg, InstantiateMsg, LockInfoResponse, Metadata, MigrateMsg, PushExecuteMsg, ReceiveMsg, VeNftCollection
};

use crate::error::ContractError;
use crate::state::{Lock, Point, BLACKLIST, CONFIG, HISTORY, LAST_SLOPE_CHANGE, LOCKED};
use crate::utils::{
    assert_blacklist, assert_not_decommissioned, assert_periods_remaining, assert_time_limits,
    calc_voting_power, cancel_scheduled_slope, fetch_last_checkpoint, fetch_slope_changes,
    schedule_slope_change, validate_received_cw20, validate_received_funds,
};


/// Contract name that is used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Creates a new contract with the specified parameters in [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let validated = msg
        .deposit_assets
        .into_iter()
        .map(|(asset, config)| -> Result<_, ContractError> {
            let asset = AssetInfoUnchecked::from_str(&asset)?.check(deps.api, None)?;

            Ok((asset, config))
        })
        .collect::<Result<HashMap<_, _>, ContractError>>()?;

    let config = Config {
        global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
        push_update_contracts: vec![],
        decommissioned: None,
        allowed_deposit_assets: validated,
    };
    CONFIG.save(deps.storage, &config)?;

    let cur_period = get_period(env.block.time.seconds())?;
    let point = Point {
        power: Uint128::zero(),
        start: cur_period,
        end: 0,
        slope: Default::default(),
        fixed: Uint128::zero(),
    };
    HISTORY.save(deps.storage, (env.contract.address.clone(), cur_period), &point)?;
    BLACKLIST.save(deps.storage, &vec![])?;

    // Store token info
    let data = TokenInfo {
        name: "Vote Escrowed".to_string(),
        symbol: "veLUNA".to_string(),
        decimals: 6,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };

    TOKEN_INFO.save(deps.storage, &data)?;
    
    let nft = VeNftCollection::default();
    nft.instantiate(deps, env, info, cw721_base::InstantiateMsg { 
        name: "veLUNA".to_string(), 
        symbol: "veLUNA".to_string(), 
        minter: env.contract.address.to_string() })?;

    Ok(Response::default())
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
        } => update_blacklist(deps, env, info, append_addrs, remove_addrs),
        ExecuteMsg::UpdateConfig {
            push_update_contracts,
            decommissioned,
            append_deposit_assets,
            remove_deposit_assets,
        } => execute_update_config(
            deps,
            info,
            push_update_contracts,
            decommissioned,
            append_deposit_assets,
            remove_deposit_assets,
        ),

        // USER
        ExecuteMsg::Withdraw {
            token_id,
        } => withdraw(deps, env, nft, info, token_id),
        
        ExecuteMsg::CreateLock {
            time,
        } => {
            let config = CONFIG.load(deps.storage)?;
            let asset = validate_received_funds(&info.funds, &config.allowed_deposit_assets)?;
            create_lock(deps, env, nft, info.sender, asset, time)
        },
        ExecuteMsg::ExtendLockTime {
            time,
            token_id,
        } => extend_lock_time(deps, env, nft, info, token_id, time),
        ExecuteMsg::ExtendLockAmount {
            token_id,
        } => {
            let config = CONFIG.load(deps.storage)?;
            let asset = validate_received_funds(&info.funds, &config.allowed_deposit_assets)?;
            deposit_for(deps, env, nft, config, asset, info.sender, token_id)
        },

        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),

        _ => Ok(nft.execute(deps, env, info, msg.into())?),
    }
}

fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    let received = Asset::cw20(info.sender, cw20_msg.amount);
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;
    let nft = VeNftCollection::default();

    match from_json(&cw20_msg.msg)? {
        ReceiveMsg::CreateLock {
            time,
        } => {
            let config = CONFIG.load(deps.storage)?;
            let asset_validated = validate_received_cw20(received, &config.allowed_deposit_assets)?;
            create_lock(deps, env, nft, sender, asset_validated, time)
        },
        ReceiveMsg::ExtendLockAmount {
            token_id,
        } => {
            let config = CONFIG.load(deps.storage)?;
            let asset = validate_received_cw20(received, &config.allowed_deposit_assets)?;
            deposit_for(deps, env, nft, config, asset, sender, token_id)
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
    add_voting_power: Option<Uint128>,
    add_amount: Option<Uint128>,
    reduce_power: Option<Uint128>,
    reduce_amount: Option<Uint128>,
    old_slope: Uint128,
    new_slope: Uint128,
) -> Result<(), ContractError> {
    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let contract_addr = env.contract.address;
    let add_voting_power = add_voting_power.unwrap_or_default();
    let add_amount = add_amount.unwrap_or_default();

    // Get last checkpoint
    let last_checkpoint = fetch_last_checkpoint(storage, &contract_addr, cur_period_key)?;
    let new_point = if let Some((_, mut point)) = last_checkpoint {
        let last_slope_change = LAST_SLOPE_CHANGE.may_load(storage)?.unwrap_or(0);
        if last_slope_change < cur_period {
            let scheduled_slope_changes =
                fetch_slope_changes(storage, last_slope_change, cur_period)?;
            // Recalculating passed points
            for (recalc_period, scheduled_change) in scheduled_slope_changes {
                point = Point {
                    power: calc_voting_power(&point, recalc_period),
                    start: recalc_period,
                    slope: point.slope.saturating_sub(scheduled_change),
                    ..point
                };
                HISTORY.save(storage, (contract_addr.clone(), recalc_period), &point)?
            }

            LAST_SLOPE_CHANGE.save(storage, &cur_period)?
        }

        let new_power = (calc_voting_power(&point, cur_period) + add_voting_power)
            .saturating_sub(reduce_power.unwrap_or_default());

        Point {
            power: new_power,
            slope: point.slope.saturating_sub(old_slope) + new_slope,
            start: cur_period,
            fixed: (point.fixed + add_amount)
                .checked_sub(reduce_amount.unwrap_or_default())
                .unwrap_or_default(),
            ..point
        }
    } else {
        Point {
            power: add_voting_power,
            slope: new_slope,
            start: cur_period,
            end: 0, // we don't use 'end' in total voting power calculations
            fixed: add_amount,
        }
    };
    HISTORY.save(storage, (contract_addr, cur_period_key), &new_point)?;
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
    addr: Addr,
    add_amount: Option<Uint128>,
    new_end: Option<u64>,
) -> Result<(), ContractError> {
    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let add_amount = add_amount.unwrap_or_default();
    let mut old_slope = Default::default();
    let mut add_voting_power = Uint128::zero();

    // Get the last user checkpoint
    let last_checkpoint = fetch_last_checkpoint(store, &addr, cur_period_key)?;
    let new_point = if let Some((_, point)) = last_checkpoint {
        let end = new_end.unwrap_or(point.end);
        let dt = end.saturating_sub(cur_period);
        let current_power = calc_voting_power(&point, cur_period);

        let new_slope = if dt != 0 {
            // always recalculate slope when the end has changed
            if end > point.end {
                // This is extend_lock_time. Recalculating user's voting power
                let mut lock = LOCKED.load(store, addr.clone())?;
                let mut new_voting_power = calc_coefficient(dt).checked_mul_uint(lock.asset)?;
                let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?; // end_vp
                                                                             // new_voting_power should always be >= current_power. saturating_sub is used for extra safety
                add_voting_power = new_voting_power.saturating_sub(current_power);
                lock.last_extend_lock_period = cur_period;
                LOCKED.save(store, addr.clone(), &lock, env.block.height)?;
                slope
            } else {
                // This is an increase in the user's lock amount
                let raw_add_voting_power = calc_coefficient(dt).checked_mul_uint(add_amount)?;
                let mut new_voting_power = current_power.checked_add(raw_add_voting_power)?;
                let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?;
                // new_voting_power should always be >= current_power. saturating_sub is used for extra safety
                add_voting_power = new_voting_power.saturating_sub(current_power);
                slope
            }
        } else {
            Uint128::zero()
        };

        // Cancel the previously scheduled slope change (same logic as in cancel_scheduled_slope)
        let last_slope_change = cancel_scheduled_slope(store, point.slope, point.end)?;

        if point.end > last_slope_change {
            // We need to subtract the slope point from the total voting power slope
            // Only if the point is still active and has not been processed/applied yet.
            old_slope = point.slope
        };

        Point {
            power: current_power + add_voting_power,
            slope: new_slope,
            start: cur_period,
            end,
            fixed: point.fixed + add_amount,
        }
    } else {
        // This error can't happen since this if-branch is intended for checkpoint creation
        let end = new_end.ok_or(ContractError::CheckpointInitializationFailed {})?;
        let dt = end - cur_period;
        add_voting_power = calc_coefficient(dt).checked_mul_uint(add_amount)?;
        let slope = adjust_vp_and_slope(&mut add_voting_power, dt)?; //add_amount
        Point {
            power: add_voting_power,
            slope,
            start: cur_period,
            end,
            fixed: add_amount,
        }
    };

    // Schedule a slope change
    schedule_slope_change(store, new_point.slope, new_point.end)?;

    HISTORY.save(store, (addr, cur_period_key), &new_point)?;

    checkpoint_total(
        store,
        env,
        Some(add_voting_power),
        Some(add_amount),
        None,
        None,
        old_slope,
        new_point.slope,
    )
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
    sender: Addr,
    asset: Asset,
    time: u64,
) -> Result<Response, ContractError> {
    assert_blacklist(deps.storage, &sender)?;
    assert_time_limits(time)?;

    let mint_response = nft.mint(deps, MessageInfo {
        sender: env.contract.address,
        funds: vec![],
    }, token_id, sender, None, Metadata {
        image: None,
        image_data: None,
        external_url: None,
        description: None,
        name: None,
        attributes: None,
        background_color: None,
        animation_url: None,
        youtube_url: None,
    })?;

    let block_period = get_period(env.block.time.seconds())?;
    let periods = get_periods_count(time);
    let end = block_period + periods;

    assert_periods_remaining(periods)?;

    let config = CONFIG.load(deps.storage)?;
    assert_not_decommissioned(&config)?;

    LOCKED.update(deps.storage, sender.clone(), env.block.height, |lock_opt| {
        if lock_opt.is_some() && !lock_opt.unwrap().asset.is_zero() {
            return Err(ContractError::LockAlreadyExists {});
        }
        Ok(Lock {
            token_id,
            asset,
            start: block_period,
            end,
            last_extend_lock_period: block_period,
        })
    })?;

    checkpoint(deps.storage, env.clone(), sender.clone(), Some(amount), Some(end))?;

    let lock_info = get_user_lock_info(deps.as_ref(), &env, sender.to_string())?;

    Ok(Response::default()
        .add_attribute("action", "veamp/create_lock")
        .add_attribute("voting_power", lock_info.voting_power.to_string())
        .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
        .add_attribute("lock_end", lock_info.end.to_string())
        .add_attributes(mint_response.attributes)
        .add_messages(get_push_update_msgs(config, sender, Ok(lock_info))?))
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
    asset: Asset,
    sender: Addr,
    token_id: Uint128,
) -> Result<Response, ContractError> {
    assert_blacklist(deps.storage, &sender)?;
    assert_not_decommissioned(&config)?;

    let mut new_end = None;
    LOCKED.update(deps.storage, sender.clone(), env.block.height, |lock_opt| match lock_opt {
        Some(mut lock) if !lock.asset.is_zero() => {
            let block_period = get_period(env.block.time.seconds())?;

            if lock.end < block_period + MIN_LOCK_PERIODS {
                lock.end = block_period + MIN_LOCK_PERIODS;
                new_end = Some(lock.end);
            }

            lock.asset += amount;
            Ok(lock)
        },
        _ => Err(ContractError::LockDoesNotExist {}),
    })?;

    checkpoint(deps.storage, env.clone(), sender.clone(), Some(amount), new_end)?;

    let lock_info = get_user_lock_info(deps.as_ref(), &env, sender.to_string())?;

    Ok(Response::default()
        .add_attribute("action", "veamp/deposit_for")
        .add_attribute("voting_power", lock_info.voting_power.to_string())
        .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
        .add_attribute("lock_end", lock_info.end.to_string())
        .add_messages(get_push_update_msgs(config, sender, Ok(lock_info))?))
}

/// Withdraws the whole amount of locked ampLP from a specific user lock.
/// If the user lock doesn't exist or if it has not yet expired, then a [`ContractError`] is returned.
fn withdraw(
    deps: DepsMut,
    env: Env,
    nft: VeNftCollection,
    info: MessageInfo,
    token_id: Uint128,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    // 'LockDoesNotExist' is thrown either when a lock does not exist in LOCKED or when a lock exists but lock.amount == 0
    let mut lock = LOCKED
        .may_load(deps.storage, sender.clone())?
        .filter(|lock| !lock.asset.is_zero())
        .ok_or(ContractError::LockDoesNotExist {})?;

    let cur_period = get_period(env.block.time.seconds())?;
    let config = CONFIG.load(deps.storage)?;
    let is_decommissioned = config.decommissioned.unwrap_or_default();

    if lock.end > cur_period && !is_decommissioned {
        Err(ContractError::LockHasNotExpired {})
    } else {
        let transfer_msg =
            native_asset(config.deposit_denom.clone(), lock.asset).into_msg(sender.clone())?;

        let amount = lock.asset;
        lock.asset = Uint128::zero();
        LOCKED.save(deps.storage, sender.clone(), &lock, env.block.height)?;

        if lock.end > cur_period {
            // early withdraw through decommissioned. Update voting power same as blacklist.
            let cur_period_key = cur_period;
            let last_checkpoint = fetch_last_checkpoint(deps.storage, &sender, cur_period_key)?;
            if let Some((_, point)) = last_checkpoint {
                // We need to checkpoint with zero power and zero slope
                HISTORY.save(
                    deps.storage,
                    (sender.clone(), cur_period_key),
                    &Point {
                        power: Uint128::zero(),
                        slope: Default::default(),
                        start: cur_period,
                        end: cur_period,
                        fixed: Uint128::zero(),
                    },
                )?;

                let cur_power = calc_voting_power(&point, cur_period);

                // User's contribution in the total voting power calculation
                let reduce_total_vp = cur_power;
                let old_slopes = point.slope;
                let old_amount = point.fixed;
                cancel_scheduled_slope(deps.storage, point.slope, point.end)?;

                checkpoint_total(
                    deps.storage,
                    env.clone(),
                    None,
                    None,
                    Some(reduce_total_vp),
                    Some(old_amount),
                    old_slopes,
                    Default::default(),
                )?;
            }
        } else {
            // We need to checkpoint and eliminate the slope influence on a future lock
            HISTORY.save(
                deps.storage,
                (sender.clone(), cur_period),
                &Point {
                    power: Uint128::zero(),
                    start: cur_period,
                    end: cur_period,
                    slope: Default::default(),
                    fixed: Uint128::zero(),
                },
            )?;

            // normal withdraw
            // removing funds needs to remove from total checkpoint aswell.
            checkpoint_total(
                deps.storage,
                env.clone(),
                None,
                None,
                None,
                Some(amount),
                Default::default(),
                Default::default(),
            )?;
        }

        let lock_info = get_user_lock_info(deps.as_ref(), &env, sender.to_string());
        let msgs = get_push_update_msgs(config, sender, lock_info)?;

        Ok(Response::default()
            .add_message(transfer_msg)
            .add_messages(msgs)
            .add_attribute("action", "veamp/withdraw"))
    }
}

fn get_push_update_msgs_multi(
    deps: Deps,
    env: Env,
    config: Config,
    sender: Vec<Addr>,
) -> StdResult<Vec<CosmosMsg>> {
    let results: Vec<CosmosMsg> = sender
        .into_iter()
        .map(|sender| {
            let lock_info = get_user_lock_info(deps, &env, sender.to_string());
            get_push_update_msgs(config.clone(), sender, lock_info)
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(results)
}

fn get_push_update_msgs(
    config: Config,
    sender: Addr,
    lock_info: Result<LockInfoResponse, ContractError>,
) -> StdResult<Vec<CosmosMsg>> {
    // only send update if lock info is available. LOCK info is never removed for any user that locked anything.
    if let Ok(lock_info) = lock_info {
        config
            .push_update_contracts
            .into_iter()
            .map(|contract| {
                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract.to_string(),
                    msg: to_json_binary(&PushExecuteMsg::UpdateVote {
                        user: sender.to_string(),
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
    info: MessageInfo,
    token_id: Uint128,
    time: u64,
) -> Result<Response, ContractError> {
    let user = info.sender;
    assert_blacklist(deps.storage, &user)?;
    let mut lock = LOCKED
        .may_load(deps.storage, user.clone())?
        .filter(|lock| !lock.asset.is_zero())
        .ok_or(ContractError::LockDoesNotExist {})?;

    // Disable the ability to extend the lock time by less than a week
    assert_time_limits(time)?;

    let block_period = get_period(env.block.time.seconds())?;
    if lock.end < block_period {
        // if the lock.end is in the past, extend_lock_time always starts from the current period.
        lock.end = block_period;
    };

    lock.end += get_periods_count(time);

    let periods = lock.end - block_period;
    assert_periods_remaining(periods)?;

    // Should not exceed MAX_LOCK_TIME
    assert_time_limits(EPOCH_START + lock.end * WEEK - env.block.time.seconds())?;

    LOCKED.save(deps.storage, user.clone(), &lock, env.block.height)?;

    checkpoint(deps.storage, env.clone(), user.clone(), None, Some(lock.end))?;

    let config = CONFIG.load(deps.storage)?;
    assert_not_decommissioned(&config)?;

    let lock_info = get_user_lock_info(deps.as_ref(), &env, user.to_string())?;

    Ok(Response::default()
        .add_attribute("action", "veamp/extend_lock_time")
        .add_attribute("voting_power", lock_info.voting_power.to_string())
        .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
        .add_attribute("lock_end", lock_info.end.to_string())
        .add_messages(get_push_update_msgs(config, user, Ok(lock_info))?))
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
    info: MessageInfo,
    append_addrs: Option<Vec<String>>,
    remove_addrs: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Permission check
    if info.sender != config.owner && Some(info.sender) != config.guardian_addr {
        return Err(ContractError::Unauthorized {});
    }
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

    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let mut reduce_total_vp = Uint128::zero(); // accumulator for decreasing total voting power
    let mut old_slopes = Uint128::zero(); // accumulator for old slopes
    let mut old_amount = Uint128::zero(); // accumulator for old amount

    let mut used_addr: HashSet<Addr> = HashSet::new();

    for addr in append.iter() {
        if !used_addr.insert(addr.clone()) {
            return Err(ContractError::AddressBlacklistDuplicated(addr.to_string()));
        }

        let last_checkpoint = fetch_last_checkpoint(deps.storage, addr, cur_period_key)?;
        if let Some((_, point)) = last_checkpoint {
            // We need to checkpoint with zero power and zero slope
            HISTORY.save(
                deps.storage,
                (addr.clone(), cur_period_key),
                &Point {
                    power: Uint128::zero(),
                    slope: Default::default(),
                    start: cur_period,
                    end: cur_period,
                    fixed: Uint128::zero(),
                },
            )?;

            let cur_power = calc_voting_power(&point, cur_period);
            // User's contribution is already zero. Skipping them
            if cur_power.is_zero() {
                continue;
            }

            // User's contribution in the total voting power calculation
            reduce_total_vp += cur_power;
            old_slopes += point.slope;
            old_amount += point.fixed;
            cancel_scheduled_slope(deps.storage, point.slope, point.end)?;
        }
    }

    if !reduce_total_vp.is_zero() || !old_slopes.is_zero() {
        // Trigger a total voting power recalculation
        checkpoint_total(
            deps.storage,
            env.clone(),
            None,
            None,
            Some(reduce_total_vp),
            Some(old_amount),
            old_slopes,
            Default::default(),
        )?;
    }

    for addr in remove.iter() {
        if !used_addr.insert(addr.clone()) {
            return Err(ContractError::AddressBlacklistDuplicated(addr.to_string()));
        }

        let lock_opt = LOCKED.may_load(deps.storage, addr.clone())?;
        if let Some(Lock {
            asset: amount,
            end,
            ..
        }) = lock_opt
        {
            checkpoint(deps.storage, env.clone(), addr.clone(), Some(amount), Some(end))?;
        }
    }

    BLACKLIST.update(deps.storage, |blacklist| -> StdResult<Vec<Addr>> {
        let mut updated_blacklist: Vec<_> =
            blacklist.into_iter().filter(|addr| !remove.contains(addr)).collect();
        updated_blacklist.extend(append.clone());
        Ok(updated_blacklist)
    })?;

    let mut attrs = vec![attr("action", "veamp/update_blacklist")];
    if !append_addrs.is_empty() {
        attrs.push(attr("added_addresses", append_addrs.join(",")))
    }
    if !remove_addrs.is_empty() {
        attrs.push(attr("removed_addresses", remove_addrs.join(",")))
    }

    Ok(Response::default()
        .add_attributes(attrs)
        .add_messages(get_push_update_msgs_multi(
            deps.as_ref(),
            env.clone(),
            config.clone(),
            append,
        )?)
        .add_messages(get_push_update_msgs_multi(deps.as_ref(), env, config, remove)?))
}

/// Updates contracts' guardian address.
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    push_update_contracts: Option<Vec<String>>,
    decommissioned: Option<bool>,
    append_deposit_assets: Option<HashMap<String, AssetInfoLockConfig>>,
    remove_deposit_assets: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.global_config().assert_owner(&deps.querier, &info.sender)?;

    if let Some(decommissioned) = decommissioned {
        if decommissioned {
            cfg.decommissioned = Some(true);
        }
    }

    if let Some(push_update_contracts) = push_update_contracts {
        cfg.push_update_contracts = push_update_contracts
            .iter()
            .map(|c| deps.api.addr_validate(c))
            .collect::<StdResult<Vec<_>>>()?;
    }

    CONFIG.save(deps.storage, &cfg)?;

    todo!("implement xxx");

    Ok(Response::default().add_attribute("action", "veamp/execute_update_config"))
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(ContractError::MigrationError(format!(
            "contract_name does not match: prev: {0}, new: {1}",
            contract_version.contract, CONTRACT_VERSION
        )));
    }

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
