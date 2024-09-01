use crate::constants::{CLAIM_REWARD_ERROR_REPLY_ID, CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::state::{ACTIONS, CONFIG, ORACLES, STATE, VALIDATORS, WALLET_ACTIONS};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  Binary, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, Uint128,
};
use cw2::set_contract_version;
use std::cmp;
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::{
  MsgClaimDelegationRewards, MsgDelegate, MsgRedelegate, MsgUndelegate,
};
use terra_proto_rs::cosmos::bank::v1beta1::Balance;
use terra_proto_rs::cosmos::base::v1beta1::Coin;
use terra_proto_rs::traits::Message;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::adapters::pair::Pair;
use ve3_shared::adapters::router::Router;
use ve3_shared::constants::{
  AT_DELEGATION_CONTROLLER, PDT_CONFIG_OWNER, PDT_CONTROLLER, PDT_VETO_CONFIG_OWNER,
};
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::helpers::assets::Assets;
use ve3_shared::helpers::denom::MsgCreateDenom;
use ve3_shared::msgs_phoenix_alliance_treasury::{
  AllianceDelegateMsg, AllianceRedelegateMsg, AllianceUndelegateMsg, Config, ExecuteMsg,
  InstantiateMsg, Oracle, State, TreasuryAction, TreasuryActionRuntime, TreasuryActionSetup,
  Validate,
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
    alliance_token_supply: vt_total_supply,
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
    } => {
      let config = CONFIG.load(deps.storage)?;
      let mut state = STATE.load(deps.storage)?;
      assert_controller(&deps, &info, &config)?;

      let (reserved, recipients, runtime) = match &action {
        TreasuryActionSetup::Payments {
          payments,
        } => {
          let mut assets = Assets::default();
          let mut recipients = vec![];

          for (recipient, asset) in payments {
            assets.add(asset);
            recipients.push(recipient);
          }

          (assets, recipients, TreasuryActionRuntime::Empty {})
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
          (Assets::from(amount), vec![recipient], TreasuryActionRuntime::Empty {})
        },
        TreasuryActionSetup::Vesting {
          recipient,
          amount,
          ..
        } => (Assets::from(amount.clone()), vec![recipient], TreasuryActionRuntime::Empty {}),
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
          setup: action.clone(),
          active_from: env.block.time.seconds() + delay,
          value_usd,
          runtime,
        },
      )?;

      // index recipients for improved queries
      for recipient in recipients {
        let addr = deps.api.addr_validate(recipient)?;
        WALLET_ACTIONS.save(deps.storage, (&addr, id), &())?;
      }

      STATE.save(deps.storage, &state)?;

      Ok(Response::new().add_attributes(vec![("action", "pdt/setup"), ("id", &id.to_string())]))
    },
    ExecuteMsg::Cancel {
      id,
    } => {
      let config = CONFIG.load(deps.storage)?;
      assert_controller(&deps, &info, &config)?;

      let state = STATE.load(deps.storage)?;
      let action = ACTIONS.load(deps.storage, id)?;
      if action.cancelled {
        return Err(ContractError::ActionCancelled(id));
      }
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
      if action.cancelled {
        return Err(ContractError::ActionCancelled(id));
      }

      if veto_config.min_amount_usd > action.value_usd {
        return Err(ContractError::ActionValueNotEnough(
          veto_config.min_amount_usd,
          action.value_usd,
        ));
      }

      cancel_action(&mut deps, state, action)?;

      Ok(Response::new().add_attributes(vec![("action", "pdt/veto"), ("id", &id.to_string())]))
    },
    ExecuteMsg::Claim {
      id,
    } => {
      let action = ACTIONS.load(deps.storage, id)?;
      if action.cancelled {
        return Err(ContractError::ActionCancelled(id));
      }

      Ok(Response::new().add_attributes(vec![("action", "pdt/claim"), ("id", &id.to_string())]))
    },
    ExecuteMsg::Execute {
      id,
    } => {
      let action = ACTIONS.load(deps.storage, id)?;
      if action.cancelled {
        return Err(ContractError::ActionCancelled(id));
      }

      Ok(Response::new().add_attributes(vec![("action", "pdt/execute"), ("id", &id.to_string())]))
    },
  }
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

fn remove_validator(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  validator: String,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;

  let mut validators = VALIDATORS.load(deps.storage)?;
  validators.remove(&validator);
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "pdt/remove_validator")]))
}

fn alliance_delegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceDelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;
  if msg.delegations.is_empty() {
    return Err(ContractError::EmptyDelegation {});
  }
  let mut validators = VALIDATORS.load(deps.storage)?;
  let mut msgs: Vec<CosmosMsg> = vec![];
  for delegation in msg.delegations {
    let delegate_msg = MsgDelegate {
      amount: Some(Coin {
        denom: config.alliance_token_denom.clone(),
        amount: delegation.amount.to_string(),
      }),
      delegator_address: env.contract.address.to_string(),
      validator_address: delegation.validator.to_string(),
    };
    msgs.push(CosmosMsg::Stargate {
      type_url: "/alliance.alliance.MsgDelegate".to_string(),
      value: Binary::from(delegate_msg.encode_to_vec()),
    });
    validators.insert(delegation.validator);
  }
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "pdt/alliance_delegate")]).add_messages(msgs))
}

fn alliance_undelegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceUndelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;
  if msg.undelegations.is_empty() {
    return Err(ContractError::EmptyDelegation {});
  }
  let mut msgs = vec![];
  for delegation in msg.undelegations {
    let undelegate_msg = MsgUndelegate {
      amount: Some(Coin {
        denom: config.alliance_token_denom.clone(),
        amount: delegation.amount.to_string(),
      }),
      delegator_address: env.contract.address.to_string(),
      validator_address: delegation.validator.to_string(),
    };
    let msg = CosmosMsg::Stargate {
      type_url: "/alliance.alliance.MsgUndelegate".to_string(),
      value: Binary::from(undelegate_msg.encode_to_vec()),
    };
    msgs.push(msg);
  }
  Ok(Response::new().add_attributes(vec![("action", "pdt/alliance_undelegate")]).add_messages(msgs))
}

fn alliance_redelegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceRedelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;
  if msg.redelegations.is_empty() {
    return Err(ContractError::EmptyDelegation {});
  }
  let mut msgs = vec![];
  let mut validators = VALIDATORS.load(deps.storage)?;
  for redelegation in msg.redelegations {
    let src_validator = redelegation.src_validator;
    let dst_validator = redelegation.dst_validator;
    let redelegate_msg = MsgRedelegate {
      amount: Some(Coin {
        denom: config.alliance_token_denom.clone(),
        amount: redelegation.amount.to_string(),
      }),
      delegator_address: env.contract.address.to_string(),
      validator_src_address: src_validator.to_string(),
      validator_dst_address: dst_validator.to_string(),
    };
    let msg = CosmosMsg::Stargate {
      type_url: "/alliance.alliance.MsgRedelegate".to_string(),
      value: Binary::from(redelegate_msg.encode_to_vec()),
    };
    msgs.push(msg);
    validators.insert(dst_validator);
  }
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "pdt/alliance_redelegate")]).add_messages(msgs))
}

fn claim_rewards(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;

  let validators = VALIDATORS.load(deps.storage)?;
  let sub_msgs: Vec<SubMsg> = validators
    .iter()
    .map(|v| {
      let msg = MsgClaimDelegationRewards {
        delegator_address: env.contract.address.to_string(),
        validator_address: v.to_string(),
        denom: config.alliance_token_denom.clone(),
      };
      let msg = CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgClaimDelegationRewards".to_string(),
        value: Binary::from(msg.encode_to_vec()),
      };
      // Reply on error here is used to ignore errors from claiming rewards with validators that we did not delegate to
      SubMsg::reply_on_error(msg, CLAIM_REWARD_ERROR_REPLY_ID)
    })
    .collect();

  Ok(
    Response::new().add_attributes(vec![("action", "pdt/claim_rewards")]).add_submessages(sub_msgs),
  )
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

// Controller is used to perform administrative operations that deals with delegating the virtual
// tokens to the expected validators
fn assert_delegation_controller(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(
    &deps.querier,
    AT_DELEGATION_CONTROLLER,
    &info.sender,
  )?;
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
