use crate::constants::{
  CLAIM_REWARD_ERROR_REPLY_ID, CONTRACT_NAME, CONTRACT_VERSION, MAX_OTC_DISCOUNT, UFACTOR,
};
use crate::domains::alliance::{
  alliance_delegate, alliance_redelegate, alliance_undelegate, claim_rewards, remove_validator,
};
use crate::error::ContractError;
use crate::state::{ACTIONS, CONFIG, ORACLES, SPENT_IN_EPOCH, STATE, USER_ACTIONS, VALIDATORS};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Reply, Response, Uint128};
use cw2::set_contract_version;
use cw_asset::{Asset, AssetInfo, AssetInfoBase, AssetInfoUnchecked};
use std::cmp;
use std::collections::HashSet;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::adapters::pair::Pair;
use ve3_shared::adapters::router::Router;
use ve3_shared::constants::{PDT_CONFIG_OWNER, PDT_CONTROLLER, PDT_DCA_EXECUTOR, SECONDS_PER_30D};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_ext::AssetExt;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::helpers::assets::Assets;
use ve3_shared::helpers::denom::MsgCreateDenom;
use ve3_shared::msgs_phoenix_treasury::{
  Config, ExecuteMsg, InstantiateMsg, MilestoneRuntime, Oracle, State, TreasuryAction,
  TreasuryActionRuntime, TreasuryActionSetup, Validate,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let vt_full_denom = format!("factory/{0}/{1}", env.contract.address, msg.alliance_token_denom);
  let vt_total_supply = Uint128::from(1_000_000_000_000_u128);
  let vt_create_msg: CosmosMsg = MsgCreateDenom {
    sender: env.contract.address.to_string(),
    subdenom: msg.alliance_token_denom.to_string(),
  }
  .into();
  let vt_mint_msg: CosmosMsg = ve3_shared::helpers::denom::MsgMint {
    sender: env.contract.address.to_string(),
    amount: Some(ve3_shared::helpers::denom::Coin {
      denom: vt_full_denom.to_string(),
      amount: vt_total_supply.to_string(),
    }),
    mint_to_address: env.contract.address.to_string(),
  }
  .into();

  let config = Config {
    alliance_token_denom: vt_full_denom.clone(),
    reward_denom: msg.reward_denom,
    global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    veto_owner: deps.api.addr_validate(&msg.veto_owner)?,
    vetos: msg.vetos.check(deps.api)?,
    allowed_actions: msg.allowed_actions,
  };

  CONFIG.save(deps.storage, &config)?;
  STATE.save(
    deps.storage,
    &State {
      reserved: Assets::default(),
      max_id: 0,
      clawback: false,
    },
  )?;

  VALIDATORS.save(deps.storage, &HashSet::new())?;

  for (asset_info, oracle) in msg.oracles {
    ORACLES.save(deps.storage, &asset_info.check(deps.api, None)?, &oracle.check(deps.api)?)?
  }

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "pdt/instantiate"),
        ("alliance_token_denom", &vt_full_denom),
        ("alliance_token_total_supply", &vt_total_supply.to_string()),
      ])
      .add_message(vt_create_msg)
      .add_message(vt_mint_msg),
  )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  match msg {
    ExecuteMsg::ClaimRewards {} => claim_rewards(deps, env, info),
    ExecuteMsg::AllianceDelegate(msg) => alliance_delegate(deps, env, info, msg),
    ExecuteMsg::AllianceUndelegate(msg) => alliance_undelegate(deps, env, info, msg),
    ExecuteMsg::AllianceRedelegate(msg) => alliance_redelegate(deps, env, info, msg),
    ExecuteMsg::RemoveValidator {
      validator,
    } => remove_validator(deps, env, info, validator),

    ExecuteMsg::UpdateConfig {
      add_oracle,
      remove_oracle,
    } => {
      let config = CONFIG.load(deps.storage)?;
      assert_config_owner(&deps, &info, &config)?;

      if let Some(add_oracle) = add_oracle {
        for (asset_info, oracle) in add_oracle {
          ORACLES.save(
            deps.storage,
            &asset_info.check(deps.api, None)?,
            &oracle.check(deps.api)?,
          )?
        }
      }

      if let Some(remove_oracle) = remove_oracle {
        for asset_info in remove_oracle {
          ORACLES.remove(deps.storage, &asset_info.check(deps.api, None)?);
        }
      }

      Ok(Response::new().add_attributes(vec![("action", "pdt/update_config")]))
    },

    ExecuteMsg::UpdateVetoConfig {
      vetos,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      assert_veto_config_owner(&deps, &info, &config)?;
      config.vetos = vetos.check(deps.api)?;
      CONFIG.save(deps.storage, &config)?;
      Ok(Response::new().add_attributes(vec![("action", "pdt/update_veto_config")]))
    },

    ExecuteMsg::Clawback {
      assets,
      recipient,
    } => execute_clawback(deps, env, info, recipient, assets),

    ExecuteMsg::Setup {
      name,
      action,
    } => execute_setup(deps, env, info, name, action),

    ExecuteMsg::Claim {
      id,
    } => execute_claim(deps, env, info, id),

    ExecuteMsg::ExecuteDca {
      id,
      min_received,
    } => execute_dca(deps, env, info, id, min_received),

    ExecuteMsg::ExecuteOtc {
      id,
      offer_amount,
    } => execute_otc(deps, env, info, id, offer_amount),

    ExecuteMsg::UpdateMilestone {
      id,
      index,
      enabled,
    } => execute_update_milestone(deps, env, info, id, index, enabled),

    ExecuteMsg::Cancel {
      id,
    } => {
      let config = CONFIG.load(deps.storage)?;
      assert_controller(&deps, &info, &config)?;

      let state = STATE.load(deps.storage)?;
      let action = ACTIONS.load(deps.storage, id)?;
      assert_not_cancelled_or_done(&action)?;
      cancel_action(&mut deps, state, action)?;
      Ok(Response::new().add_attributes(vec![("action", "pdt/cancel"), ("id", &id.to_string())]))
    },

    ExecuteMsg::Veto {
      id,
    } => execute_veto(deps, env, info, id),
  }
}

fn execute_clawback(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  recipient: String,
  assets: Vec<AssetInfoUnchecked>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let mut state = STATE.load(deps.storage)?;
  assert_veto_config_owner(&deps, &info, &config)?;

  state.clawback = true;
  STATE.save(deps.storage, &state)?;

  let recipient = deps.api.addr_validate(&recipient)?;

  let mut msgs = vec![];
  for asset in assets {
    let asset = asset.check(deps.api, None)?;
    msgs.push(
      asset
        .with_balance_query(&deps.querier, &env.contract.address)?
        .transfer_msg(recipient.clone())?,
    )
  }

  Ok(Response::new().add_attributes(vec![("action", "pdt/clawback")]).add_messages(msgs))
}

fn execute_veto(
  mut deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  id: u64,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let veto_config = config
    .vetos
    .into_iter()
    .find(|a| a.vetoer == info.sender)
    .ok_or(ContractError::NotVetoer(info.sender.to_string()))?;

  let state = STATE.load(deps.storage)?;
  let action = ACTIONS.load(deps.storage, id)?;
  assert_not_cancelled_or_done(&action)?;

  if veto_config.spend_above_usd > action.total_usd
    && veto_config.spend_above_usd_30d > action.total_usd_30d
  {
    return Err(ContractError::ActionValueNotEnough(veto_config.spend_above_usd, action.total_usd));
  }

  cancel_action(&mut deps, state, action)?;

  Ok(Response::new().add_attributes(vec![("action", "pdt/veto"), ("id", &id.to_string())]))
}

fn execute_update_milestone(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  id: u64,
  index: u64,
  enabled: bool,
) -> Result<Response, ContractError> {
  let state = STATE.load(deps.storage)?;
  assert_not_clawback(&state)?;

  let config = CONFIG.load(deps.storage)?;
  let mut action = ACTIONS.load(deps.storage, id)?;
  assert_not_cancelled_or_done(&action)?;
  assert_controller(&deps, &info, &config)?;

  match (&action.setup, &action.runtime) {
    (
      TreasuryActionSetup::Milestone {
        ..
      },
      TreasuryActionRuntime::Milestone {
        milestones,
      },
    ) => {
      let mut new_milestones = milestones.clone();
      let relevant = new_milestones.get_mut(index as usize);

      match relevant {
        Some(relevant) => {
          if relevant.claimed {
            return Err(ContractError::MilestoneClaimed);
          } else {
            relevant.enabled = enabled;
          }
        },
        None => Err(ContractError::MilestoneNotFound)?,
      }

      action.runtime = TreasuryActionRuntime::Milestone {
        milestones: new_milestones,
      };
      ACTIONS.save(deps.storage, id, &action)?;

      Ok(
        Response::new()
          .add_attributes(vec![("action", "pdt/update_milestone"), ("id", &id.to_string())]),
      )
    },
    _ => Err(ContractError::CannotExecuteOnlyMilestone),
  }
}

fn execute_dca(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  id: u64,
  min_received: Option<Uint128>,
) -> Result<Response, ContractError> {
  let mut state = STATE.load(deps.storage)?;
  assert_not_clawback(&state)?;

  let config = CONFIG.load(deps.storage)?;
  let mut action = ACTIONS.load(deps.storage, id)?;
  assert_dca_executor(&deps, &info, &config)?;
  assert_not_cancelled_or_done(&action)?;
  assert_action_active(&env, &action)?;

  match (&action.setup, &action.runtime) {
    (
      TreasuryActionSetup::Dca {
        amount,
        into,
        max_per_swap,
        start_s: start_unix_s,
        end_s: end_unix_s,
        cooldown_s,
      },
      TreasuryActionRuntime::Dca {
        last_execution_s: last_execution_unix_s,
      },
    ) => {
      let from = *cmp::max(start_unix_s, last_execution_unix_s);
      let to = cmp::min(env.block.time.seconds(), *end_unix_s);

      if env.block.time.seconds() < *start_unix_s {
        return Err(ContractError::CannotExecuteDcaNotActive);
      }

      if last_execution_unix_s + cooldown_s > env.block.time.seconds() {
        return Err(ContractError::DcaWaitForCooldown(last_execution_unix_s + cooldown_s));
      }

      let remaining = action.reserved.get(&amount.info).map(|a| a.amount).unwrap_or_default();

      let mut use_amount = if to == *end_unix_s {
        // end already reached -> use remaining reserved of action
        action.runtime = TreasuryActionRuntime::Dca {
          last_execution_s: env.block.time.seconds(),
        };

        remaining
      } else {
        let delta = end_unix_s - start_unix_s;
        let distance = to - from;
        let send_amount = amount.amount.multiply_ratio(distance, delta);

        action.runtime = TreasuryActionRuntime::Dca {
          last_execution_s: env.block.time.seconds(),
        };

        send_amount
      };

      if let Some(max_per_swap) = max_per_swap {
        use_amount = cmp::min(*max_per_swap, use_amount)
      }

      let use_asset = amount.info.with_balance(use_amount);

      state.reserved.remove(&use_asset)?;
      STATE.save(deps.storage, &state)?;

      action.reserved.remove(&use_asset)?;
      action.done = action.reserved.0.is_empty();
      ACTIONS.save(deps.storage, id, &action)?;

      let swap_msgs = config.zapper(&deps.querier)?.swap_msgs(
        into.into(),
        vec![use_asset.clone()],
        min_received,
        None,
      )?;

      Ok(
        Response::new()
          .add_attributes(vec![
            ("action", "pdt/execute_dca"),
            ("id", &id.to_string()),
            ("offer", &use_asset.to_string()),
          ])
          .add_messages(swap_msgs),
      )
    },
    _ => Err(ContractError::CannotExecuteOnlyDca),
  }
}

fn execute_otc(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  id: u64,
  offer_amount: Uint128,
) -> Result<Response, ContractError> {
  let mut state = STATE.load(deps.storage)?;
  assert_not_clawback(&state)?;

  let mut action = ACTIONS.load(deps.storage, id)?;
  assert_not_cancelled_or_done(&action)?;
  assert_action_active(&env, &action)?;

  match (&action.setup, &action.runtime) {
    (
      TreasuryActionSetup::Otc {
        amount,
        into,
      },
      TreasuryActionRuntime::Otc {},
    ) => {
      let remaining = action.reserved.get(&amount.info).map(|a| a.amount).unwrap_or_default();

      if offer_amount.is_zero() {
        return Err(ContractError::CannotExecuteMissingFunds);
      }

      let mut msgs: Vec<CosmosMsg> = vec![];
      let expected = into.info.with_balance(offer_amount);

      match &expected.info {
        AssetInfoBase::Native(_) => expected.assert_sent(&info)?,
        AssetInfoBase::Cw20(_) => {
          msgs.push(expected.transfer_from_msg(info.sender.clone(), env.contract.address)?)
        },
        _ => todo!(),
      }

      let return_amount = offer_amount.multiply_ratio(amount.amount, into.amount);

      if return_amount > remaining {
        return Err(ContractError::OtcAmountBiggerThanAvailable(return_amount, remaining));
      }

      let return_asset = amount.info.with_balance(return_amount);

      state.reserved.remove(&return_asset)?;
      STATE.save(deps.storage, &state)?;

      action.reserved.remove(&return_asset)?;
      action.done = action.reserved.0.is_empty();
      ACTIONS.save(deps.storage, id, &action)?;

      msgs.push(return_asset.transfer_msg(info.sender)?);

      Ok(
        Response::new()
          .add_attributes(vec![
            ("action", "pdt/execute_otc"),
            ("id", &id.to_string()),
            ("returned", &return_amount.to_string()),
          ])
          .add_messages(msgs),
      )
    },
    _ => Err(ContractError::CannotExecuteOnlyOtc),
  }
}

fn execute_setup(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  name: String,
  action: TreasuryActionSetup,
) -> Result<Response, ContractError> {
  let mut state = STATE.load(deps.storage)?;
  assert_not_clawback(&state)?;

  let config = CONFIG.load(deps.storage)?;
  assert_controller(&deps, &info, &config)?;

  assert_allowed_action(&config, &action)?;

  let (reserved, recipients, runtime) = match &action {
    TreasuryActionSetup::Payment {
      payments,
    } => {
      let mut assets = Assets::default();
      let mut recipients = vec![];

      for payment in payments {
        assets.add(&payment.asset);
        recipients.push(&payment.recipient);
      }

      (
        assets,
        recipients,
        TreasuryActionRuntime::Payment {
          open: payments.clone(),
        },
      )
    },
    TreasuryActionSetup::Otc {
      amount,
      into,
    } => {
      if amount.info == into.info {
        return Err(ContractError::SwapAssetsSame);
      }
      into.info.check(deps.api)?;

      let from_value = calculate_value_usd(&deps, &amount.clone().into())?;
      let to_value = calculate_value_usd(&deps, &into.clone().into())?;

      if to_value < MAX_OTC_DISCOUNT * from_value {
        return Err(ContractError::OtcDiscountTooHigh(MAX_OTC_DISCOUNT));
      }

      (Assets::from(amount.clone()), vec![], TreasuryActionRuntime::Otc {})
    },

    TreasuryActionSetup::Dca {
      amount,
      start_s: start_unix_s,
      into,
      ..
    } => {
      if amount.info == *into {
        return Err(ContractError::SwapAssetsSame);
      }

      into.check(deps.api)?;

      (
        Assets::from(amount.clone()),
        vec![],
        TreasuryActionRuntime::Dca {
          last_execution_s: *start_unix_s,
        },
      )
    },

    TreasuryActionSetup::Milestone {
      recipient,
      milestones,
      asset_info,
    } => {
      let asset = asset_info.with_balance(milestones.iter().map(|a| a.amount).sum());
      (
        Assets::from(asset),
        vec![recipient],
        TreasuryActionRuntime::Milestone {
          milestones: milestones
            .iter()
            .map(|a| MilestoneRuntime {
              amount: a.amount,
              enabled: false,
              claimed: false,
            })
            .collect(),
        },
      )
    },
    TreasuryActionSetup::Vesting {
      recipient,
      amount,
      ..
    } => (
      Assets::from(amount.clone()),
      vec![recipient],
      TreasuryActionRuntime::Vesting {
        last_claim_s: 0,
      },
    ),
  };

  if reserved.is_empty() {
    return Err(ContractError::ActionNotReservingAnyFunds);
  }

  let id = state.max_id + 1;
  state.max_id = id;
  state.reserved.add_multi(&reserved.0);

  // index recipients for improved queries
  for recipient in recipients {
    let addr = deps.api.addr_validate(recipient)?;
    USER_ACTIONS.save(deps.storage, (&addr, id), &())?;
  }

  // check that contract has enough assets
  for asset in &reserved.0 {
    if asset.info == AssetInfo::native(config.alliance_token_denom.clone()) {
      return Err(ContractError::CannotUseVt);
    }

    // this validates that the asset is correct
    let balance = asset.info.query_balance(&deps.querier, env.contract.address.clone())?;
    let required = state
      .reserved
      .get(&asset.info)
      .ok_or(ContractError::ExpectedAssetReservation(asset.info.clone()))?;

    if balance < required.amount {
      return Err(ContractError::NotEnoughFunds(balance, required));
    }
  }

  // calculate usd value
  let value_usd = calculate_value_usd(&deps, &reserved)?;

  let epoch_30d = env.block.time.seconds() / SECONDS_PER_30D;
  let mut spent_in_month = SPENT_IN_EPOCH.may_load(deps.storage, epoch_30d)?.unwrap_or_default();
  spent_in_month += value_usd;
  SPENT_IN_EPOCH.save(deps.storage, epoch_30d, &spent_in_month)?;

  // get delay by usd value
  let mut delay = 0u64;
  for veto in config.vetos {
    if value_usd >= veto.spend_above_usd || spent_in_month >= veto.spend_above_usd_30d {
      delay = cmp::max(veto.delay_s, delay);
    }
  }

  // save action
  ACTIONS.save(
    deps.storage,
    id,
    &TreasuryAction {
      id,
      name,
      reserved,
      cancelled: false,
      done: false,
      setup: action.clone(),
      active_from: env.block.time.seconds() + delay,
      total_usd: value_usd,
      total_usd_30d: spent_in_month,
      runtime,
    },
  )?;

  STATE.save(deps.storage, &state)?;

  Ok(Response::new().add_attributes(vec![("action", "pdt/setup"), ("id", &id.to_string())]))
}

fn assert_allowed_action(
  config: &Config,
  action: &TreasuryActionSetup,
) -> Result<(), ContractError> {
  if let Some(allowed) = &config.allowed_actions {
    if !allowed.contains(&action.to_action_str()) {
      return Err(ContractError::ActionNotAllowed);
    }
  }

  Ok(())
}

fn assert_not_clawback(state: &State) -> Result<(), ContractError> {
  if state.clawback {
    return Err(ContractError::ClawbackTriggered);
  }
  Ok(())
}

fn execute_claim(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  id: u64,
) -> Result<Response, ContractError> {
  let mut state = STATE.load(deps.storage)?;
  assert_not_clawback(&state)?;

  let mut action = ACTIONS.load(deps.storage, id)?;
  assert_not_cancelled_or_done(&action)?;
  assert_action_active(&env, &action)?;

  let send_asset: Asset;
  let send_to: String;

  match (&action.setup, &mut action.runtime) {
    (
      TreasuryActionSetup::Payment {
        ..
      },
      TreasuryActionRuntime::Payment {
        open,
      },
    ) => {
      let position = open
        .iter()
        .position(|a| a.recipient == info.sender && is_claimable(&env, a.claimable_after_s))
        .ok_or(ContractError::CannotClaimNoOpenPayment)?;

      let description = open.remove(position);

      send_asset = description.asset;
      send_to = description.recipient;
    },

    (
      TreasuryActionSetup::Milestone {
        recipient,
        asset_info: asset,
        ..
      },
      TreasuryActionRuntime::Milestone {
        milestones,
      },
    ) => {
      let mut send_amount = Uint128::zero();

      let mut new_milestones = milestones.clone();

      for milestone in new_milestones.iter_mut() {
        if milestone.enabled && !milestone.claimed {
          send_amount = send_amount.checked_add(milestone.amount)?;
          milestone.claimed = true;
        }
      }

      action.runtime = TreasuryActionRuntime::Milestone {
        milestones: new_milestones,
      };

      send_asset = asset.with_balance(send_amount);
      send_to = recipient.clone();
    },

    (
      TreasuryActionSetup::Vesting {
        recipient,
        amount,
        start_s: start_unix_s,
        end_s: end_unix_s,
      },
      TreasuryActionRuntime::Vesting {
        last_claim_s: last_claim_unix_s,
      },
    ) => {
      let from = *cmp::max(start_unix_s, last_claim_unix_s);
      let to = cmp::min(env.block.time.seconds(), *end_unix_s);

      if to <= from {
        return Err(ContractError::CannotClaimVestingNotActive);
      }

      if to == *end_unix_s {
        // end already reached -> send remaining reserved of action
        let remaining = action.reserved.get(&amount.info).map(|a| a.amount).unwrap_or_default();

        action.runtime = TreasuryActionRuntime::Vesting {
          last_claim_s: env.block.time.seconds(),
        };

        send_asset = amount.info.with_balance(remaining);
        send_to = recipient.clone();
      } else {
        let delta = end_unix_s - start_unix_s;
        let distance = to - from;
        let send_amount = amount.amount.multiply_ratio(distance, delta);

        action.runtime = TreasuryActionRuntime::Vesting {
          last_claim_s: env.block.time.seconds(),
        };

        send_asset = amount.info.with_balance(send_amount);
        send_to = recipient.clone();
      }
    },
    (_, _) => return Err(ContractError::CannotClaimNotAllowed),
  }

  if send_asset.amount.is_zero() {
    return Err(ContractError::CannotClaimNothingToClaim);
  }

  state.reserved.remove(&send_asset)?;
  STATE.save(deps.storage, &state)?;

  action.reserved.remove(&send_asset)?;
  action.done = action.reserved.0.is_empty();

  ACTIONS.save(deps.storage, id, &action)?;

  let transfer_msg = send_asset.transfer_msg(send_to)?;

  Ok(
    Response::new()
      .add_attributes(vec![("action", "pdt/claim"), ("id", &id.to_string())])
      .add_message(transfer_msg),
  )
}

fn assert_action_active(env: &Env, action: &TreasuryAction) -> Result<(), ContractError> {
  if env.block.time.seconds() < action.active_from {
    return Err(ContractError::CannotExecuteNotActive);
  }
  Ok(())
}

fn is_claimable(env: &Env, claimable_after_unix_s: Option<u64>) -> bool {
  match claimable_after_unix_s {
    Some(claimable_after_unix_s) => env.block.time.seconds() >= claimable_after_unix_s,
    None => true,
  }
}

fn assert_not_cancelled_or_done(action: &TreasuryAction) -> Result<(), ContractError> {
  if action.cancelled {
    return Err(ContractError::ActionCancelled(action.id));
  }

  if action.done {
    return Err(ContractError::ActionDone(action.id));
  }

  Ok(())
}

fn calculate_value_usd(deps: &DepsMut, reserved: &Assets) -> Result<Uint128, ContractError> {
  let mut value_uusd = Uint128::zero();

  for asset in &reserved.0 {
    let oracle = ORACLES
      .load(deps.storage, &asset.info)
      .map_err(|_| ContractError::MissingOracle(asset.info.clone()))?;

    let added_usd = match oracle {
      Oracle::Usdc => asset.amount,
      Oracle::Pair {
        contract,
        simulation_amount,
        ..
      } => {
        let result = Pair(contract).query_simulate(
          &deps.querier,
          asset.info.with_balance(simulation_amount),
          None,
        )?;

        let price = Decimal::from_ratio(result.return_amount, simulation_amount);
        price * asset.amount
        // Factor not needed as asset.amount already has it.
        // * Decimal::from_ratio(u32::pow(10, from_decimals.unwrap_or(6)), u32::pow(10, 6))
      },

      Oracle::Route {
        contract,
        path,
        simulation_amount,
        ..
      } => {
        let result = Router(contract).query_simulate(
          &deps.querier,
          asset.info.with_balance(simulation_amount),
          path,
        )?;

        let price = Decimal::from_ratio(result.amount, simulation_amount);
        price * asset.amount
        // Factor not needed as asset.amount already has it.
        // * Decimal::from_ratio(u32::pow(10, from_decimals.unwrap_or(6)), u32::pow(10, 6))
      },
    };

    if added_usd.is_zero() {
      return Err(ContractError::OracleReturnedZeroUsd(asset.clone()));
    }

    value_uusd = value_uusd.checked_add(added_usd)?;
  }

  let value_usd = value_uusd.multiply_ratio(Uint128::one(), UFACTOR);

  // println!("value_uusd: {value_uusd}");
  // println!("value_usd: {value_usd}");
  Ok(value_usd)
}

fn cancel_action(
  deps: &mut DepsMut,
  mut state: State,
  mut action: TreasuryAction,
) -> Result<(), ContractError> {
  // free up the reserved assets
  state.reserved.remove_multi(&action.reserved.0)?;
  action.cancelled = true;
  action.reserved = Assets(vec![]);

  STATE.save(deps.storage, &state)?;
  ACTIONS.save(deps.storage, action.id, &action)?;
  Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
  match reply.id {
    CLAIM_REWARD_ERROR_REPLY_ID => {
      Ok(Response::new().add_attributes(vec![("action", "pdt/claim_reward_error")]))
    },
    _ => Err(ContractError::InvalidReplyId(reply.id)),
  }
}

fn assert_controller(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(&deps.querier, PDT_CONTROLLER, &info.sender)?;
  Ok(())
}

fn assert_dca_executor(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(&deps.querier, PDT_DCA_EXECUTOR, &info.sender)?;
  Ok(())
}

fn assert_veto_config_owner(
  _deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  if config.veto_owner != info.sender {
    return Err(ContractError::SharedError(SharedError::Unauthorized {}));
  }

  Ok(())
}

fn assert_config_owner(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(&deps.querier, PDT_CONFIG_OWNER, &info.sender)?;
  Ok(())
}
