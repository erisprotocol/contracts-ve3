use crate::constants::{CLAIM_REWARD_ERROR_REPLY_ID, CONTRACT_NAME, CONTRACT_VERSION};
use crate::domains::alliance::{
  alliance_delegate, alliance_redelegate, alliance_undelegate, claim_rewards, remove_validator,
};
use crate::error::ContractError;
use crate::state::{ACTIONS, CONFIG, ORACLES, STATE, USER_ACTIONS, VALIDATORS};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Reply, Response, Uint128};
use cw2::set_contract_version;
use cw_asset::Asset;
use std::cmp;
use std::collections::HashSet;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::adapters::pair::Pair;
use ve3_shared::adapters::router::Router;
use ve3_shared::constants::{PDT_CONFIG_OWNER, PDT_CONTROLLER, PDT_VETO_CONFIG_OWNER};
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
    vetos: msg.vetos.check(deps.api)?,
  };

  CONFIG.save(deps.storage, &config)?;
  STATE.save(
    deps.storage,
    &State {
      reserved: Assets::default(),
      max_id: 0,
    },
  )?;

  VALIDATORS.save(deps.storage, &HashSet::new())?;
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

    ExecuteMsg::Setup {
      name,
      action,
    } => execute_setup(deps, env, info, name, action),

    ExecuteMsg::Claim {
      id,
    } => execute_claim(deps, env, info, id),

    ExecuteMsg::Execute {
      id,
      min_received,
    } => execute_dca(deps, env, info, id, min_received),

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
    } => {
      let config = CONFIG.load(deps.storage)?;
      let veto_config = config
        .vetos
        .into_iter()
        .find(|a| a.vetoer == info.sender)
        .ok_or(ContractError::NotVetoer(info.sender.to_string()))?;

      let state = STATE.load(deps.storage)?;
      let action = ACTIONS.load(deps.storage, id)?;
      assert_not_cancelled_or_done(&action)?;

      if veto_config.min_amount_usd > action.value_usd {
        return Err(ContractError::ActionValueNotEnough(
          veto_config.min_amount_usd,
          action.value_usd,
        ));
      }

      cancel_action(&mut deps, state, action)?;

      Ok(Response::new().add_attributes(vec![("action", "pdt/veto"), ("id", &id.to_string())]))
    },
  }
}

fn execute_dca(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  id: u64,
  min_received: Option<Uint128>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let mut action = ACTIONS.load(deps.storage, id)?;
  assert_not_cancelled_or_done(&action)?;

  match (&action.setup, &action.runtime) {
    (
      TreasuryActionSetup::Dca {
        amount,
        into,
        max_per_swap,
        start_unix_s,
        end_unix_s,
      },
      TreasuryActionRuntime::Dca {
        last_execution_unix_s,
      },
    ) => {
      let from = *cmp::max(start_unix_s, last_execution_unix_s);
      let to = cmp::min(env.block.time.seconds(), *end_unix_s);

      if to <= from {
        return Err(ContractError::CannotExecute("DCA not yet active".to_string()));
      }

      let remaining = action.reserved.get(&amount.info).map(|a| a.amount).unwrap_or_default();

      let mut use_amount = if to == *end_unix_s {
        // end already reached -> use remaining reserved of action
        action.runtime = TreasuryActionRuntime::Vesting {
          last_claim_unix_s: env.block.time.seconds(),
        };

        remaining
      } else {
        let delta = end_unix_s - start_unix_s;
        let distance = to - from;
        let send_amount = amount.amount.multiply_ratio(distance, delta);

        action.runtime = TreasuryActionRuntime::Vesting {
          last_claim_unix_s: env.block.time.seconds(),
        };

        send_amount
      };

      if let Some(max_per_swap) = max_per_swap {
        use_amount = cmp::min(*max_per_swap, use_amount)
      }

      let use_asset = amount.info.with_balance(use_amount);

      let mut state = STATE.load(deps.storage)?;
      state.reserved.remove(&use_asset)?;
      STATE.save(deps.storage, &state)?;

      action.reserved.remove(&use_asset)?;
      action.done = action.reserved.0.is_empty();
      ACTIONS.save(deps.storage, id, &action)?;

      let swap_msgs = config.zapper(&deps.querier)?.swap_msgs(
        into.into(),
        vec![use_asset],
        min_received,
        None,
      )?;

      Ok(
        Response::new()
          .add_attributes(vec![("action", "pdt/execute_dca"), ("id", &id.to_string())])
          .add_messages(swap_msgs),
      )
    },
    _ => Err(ContractError::CannotExecute("only available for DCA".to_string())),
  }
}

fn execute_setup(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  name: String,
  action: TreasuryActionSetup,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let mut state = STATE.load(deps.storage)?;
  assert_controller(&deps, &info, &config)?;

  let (reserved, recipients, runtime) = match &action {
    TreasuryActionSetup::Payment {
      payments,
    } => {
      let mut assets = Assets::default();
      let mut recipients = vec![];

      for (recipient, asset) in payments {
        assets.add(asset);
        recipients.push(recipient);
      }

      (
        assets,
        recipients,
        TreasuryActionRuntime::Payment {
          open: payments.clone(),
        },
      )
    },

    TreasuryActionSetup::Dca {
      amount,
      ..
    } => (Assets::from(amount.clone()), vec![], TreasuryActionRuntime::Empty {}),

    TreasuryActionSetup::Milestone {
      recipient,
      milestones,
      asset,
    } => {
      let amount = asset.with_balance(milestones.iter().map(|a| a.amount).sum());
      (
        Assets::from(amount),
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
        last_claim_unix_s: 0,
      },
    ),
  };

  let id = state.max_id + 1;
  state.max_id = id;
  state.reserved.add_multi(&reserved.0);

  // check that contract has enough assets
  for asset in &reserved.0 {
    let balance = asset.info.query_balance(&deps.querier, env.contract.address.clone())?;
    let required = state
      .reserved
      .get(&asset.info)
      .ok_or(ContractError::ExpectedAssetReservation(asset.info.clone()))?;

    if balance < required.amount {
      return Err(ContractError::NotEnoughBalance(balance, required));
    }
  }

  // calculate usd value
  let value_usd = calculate_value(&deps, &reserved)?;

  // get delay by usd value
  let mut delay = 0u64;
  for veto in config.vetos {
    if value_usd >= veto.min_amount_usd {
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
      claim_active_from: env.block.time.seconds() + delay,
      value_usd,
      runtime,
    },
  )?;

  // index recipients for improved queries
  for recipient in recipients {
    let addr = deps.api.addr_validate(recipient)?;
    USER_ACTIONS.save(deps.storage, (&addr, id), &())?;
  }

  STATE.save(deps.storage, &state)?;

  Ok(Response::new().add_attributes(vec![("action", "pdt/setup"), ("id", &id.to_string())]))
}

fn execute_claim(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  id: u64,
) -> Result<Response, ContractError> {
  let mut action = ACTIONS.load(deps.storage, id)?;

  assert_not_cancelled_or_done(&action)?;

  if env.block.time.seconds() < action.claim_active_from {
    return Err(ContractError::CannotClaim("not active".to_string()));
  }

  let send_asset: Asset;
  let send_to: String;

  match (&action.setup, &action.runtime) {
    (
      TreasuryActionSetup::Payment {
        ..
      },
      TreasuryActionRuntime::Payment {
        open,
      },
    ) => {
      let (recipient, asset) = open
        .iter()
        .find(|a| a.0 == info.sender)
        .ok_or(ContractError::CannotClaim("no open payment for sender".to_string()))?;

      send_asset = asset.clone();
      send_to = recipient.clone();
    },

    (
      TreasuryActionSetup::Milestone {
        recipient,
        asset,
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
        start_unix_s,
        end_unix_s,
      },
      TreasuryActionRuntime::Vesting {
        last_claim_unix_s,
      },
    ) => {
      let from = *cmp::max(start_unix_s, last_claim_unix_s);
      let to = cmp::min(env.block.time.seconds(), *end_unix_s);

      if to <= from {
        return Err(ContractError::CannotClaim("vesting not yet active".to_string()));
      }

      if to == *end_unix_s {
        // end already reached -> send remaining reserved of action
        let remaining = action.reserved.get(&amount.info).map(|a| a.amount).unwrap_or_default();

        action.runtime = TreasuryActionRuntime::Vesting {
          last_claim_unix_s: env.block.time.seconds(),
        };

        send_asset = amount.info.with_balance(remaining);
        send_to = recipient.clone();
      } else {
        let delta = end_unix_s - start_unix_s;
        let distance = to - from;
        let send_amount = amount.amount.multiply_ratio(distance, delta);

        action.runtime = TreasuryActionRuntime::Vesting {
          last_claim_unix_s: env.block.time.seconds(),
        };

        send_asset = amount.info.with_balance(send_amount);
        send_to = recipient.clone();
      }
    },
    (_, _) => return Err(ContractError::CannotClaim("not allowed".to_string())),
  }

  let mut state = STATE.load(deps.storage)?;
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

fn assert_not_cancelled_or_done(action: &TreasuryAction) -> Result<(), ContractError> {
  if action.cancelled {
    return Err(ContractError::ActionCancelled(action.id));
  }

  if action.done {
    return Err(ContractError::ActionDone(action.id));
  }

  Ok(())
}

fn calculate_value(deps: &DepsMut, reserved: &Assets) -> Result<Uint128, ContractError> {
  let mut value_usd = Uint128::zero();

  for asset in &reserved.0 {
    let oracle = ORACLES.load(deps.storage, &asset.info)?;

    let added_usd = match oracle {
      Oracle::Usdc => asset.amount,
      Oracle::Pair {
        contract,
        simulation_amount,
      } => {
        let result = Pair(contract).query_simulate(
          &deps.querier,
          asset.info.with_balance(simulation_amount),
          None,
        )?;

        let price = Decimal::from_ratio(result.return_amount, simulation_amount);
        price * asset.amount
      },

      Oracle::Route {
        contract,
        path,
        simulation_amount,
      } => {
        let result = Router(contract).query_simulate(
          &deps.querier,
          asset.info.with_balance(simulation_amount),
          path,
        )?;

        let price = Decimal::from_ratio(result.amount, simulation_amount);
        price * asset.amount
      },
    };

    if added_usd.is_zero() {
      return Err(ContractError::OracleReturnedZeroUsd(asset.clone()));
    }

    value_usd = value_usd.checked_add(added_usd)?;
  }

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
fn assert_veto_config_owner(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(&deps.querier, PDT_VETO_CONFIG_OWNER, &info.sender)?;
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
