use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  attr, entry_point, from_json, Addr, Api, Attribute, Binary, CosmosMsg, Deps, DepsMut, Env,
  MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};
use cw_storage_plus::{Item, Map};
use ve3_shared::{
  constants::SECONDS_PER_WEEK, error::SharedError, extensions::asset_info_ext::AssetInfoExt,
  helpers::assets::Assets,
};
pub type ContractResult = Result<Response, SharedError>;

#[cw_serde]
pub enum ExecuteMsg {
  ClaimRewards {
    /// The LP token cw20 address or token factory denom
    lp_tokens: Vec<String>,
  },
  /// Receives a message of type [`Cw20ReceiveMsg`]. Handles cw20 LP token deposits.
  Receive(Cw20ReceiveMsg),
  /// Stake LP tokens in the Generator. LP tokens staked on behalf of recipient if recipient is set.
  /// Otherwise LP tokens are staked on behalf of message sender.
  Deposit {
    recipient: Option<String>,
  },
  /// Withdraw LP tokens from the Generator
  Withdraw {
    /// The LP token cw20 address or token factory denom
    lp_token: String,
    /// The amount to withdraw. Must not exceed total staked amount.
    amount: Uint128,
  },
}

impl fmt::Display for ExecuteMsg {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "{}",
      format!("{:?}", self).to_lowercase().chars().take_while(|&ch| ch != ' ').collect::<String>()
    )
  }
}

#[cw_serde]
/// Cw20 hook message template
pub enum Cw20Msg {
  Deposit {
    recipient: Option<String>,
  },
  /// Besides this enum variant is redundant we keep this for backward compatibility with old pair contracts
  DepositFor(String),
}

#[cw_serde]
pub struct InstantiateMsg {
  pub config: Config,
}

#[cw_serde]
pub enum QueryMsg {}

#[cw_serde]
pub struct Config {
  pub emission: AssetInfo,
  pub per_week: Uint128,
  pub per_week_xxx: Uint128,
}

#[entry_point]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  CONFIG.save(deps.storage, &msg.config)?;
  Ok(Response::new())
}

const DEPOSITS: Map<Addr, Assets> = Map::new("deposits");
const TOTAL: Item<Assets> = Item::new("totals");
const LAST_CLAIM: Map<(Addr, &AssetInfo), u64> = Map::new("last_claim");
const CONFIG: Item<Config> = Item::new("config");

#[entry_point]
pub fn execute(mut deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  let mut msgs: Vec<CosmosMsg> = vec![];
  let mut attrs: Vec<Attribute> = vec![];

  match msg.clone() {
    ExecuteMsg::ClaimRewards {
      lp_tokens,
    } => {
      if lp_tokens.len() != 1 {
        Err(StdError::generic_err("too many lp tokens - only claim 1"))?;
      }
      let asset_info = determine_asset_info(&lp_tokens[0].clone(), deps.api)?;

      _claim(&mut msgs, &mut deps, &env, info.sender, &asset_info)?;
    },

    ExecuteMsg::Receive(cw20) => {
      let recipient = match from_json(&cw20.msg)? {
        Cw20Msg::Deposit {
          recipient,
        } => recipient,
        Cw20Msg::DepositFor(recipient) => Some(recipient),
      };

      let recipient = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?
      } else {
        deps.api.addr_validate(&cw20.sender)?
      };

      let asset = Asset::cw20(info.sender, cw20.amount);

      _claim(&mut msgs, &mut deps, &env, recipient.clone(), &asset.info)?;

      let mut deposits = DEPOSITS.load(deps.storage, recipient.clone()).unwrap_or_default();
      deposits.add(&asset);
      DEPOSITS.save(deps.storage, recipient, &deposits)?;

      let mut totals = TOTAL.load(deps.storage).unwrap_or_default();
      totals.add(&asset);
      TOTAL.save(deps.storage, &totals)?;

      attrs.push(attr("mock/amount", asset.to_string()));
    },

    ExecuteMsg::Deposit {
      recipient,
    } => {
      let recipient = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?
      } else {
        info.sender
      };

      if info.funds.len() != 1 {
        Err(StdError::generic_err("funds wrong"))?;
      }

      let asset: Asset = info.funds[0].clone().into();

      _claim(&mut msgs, &mut deps, &env, recipient.clone(), &asset.info)?;

      let mut deposits = DEPOSITS.load(deps.storage, recipient.clone()).unwrap_or_default();
      deposits.add(&asset);
      DEPOSITS.save(deps.storage, recipient, &deposits)?;

      let mut totals = TOTAL.load(deps.storage).unwrap_or_default();
      totals.add(&asset);
      TOTAL.save(deps.storage, &totals)?;

      attrs.push(attr("mock/amount", asset.to_string()));
    },

    ExecuteMsg::Withdraw {
      lp_token,
      amount,
    } => {
      let asset_info = determine_asset_info(&lp_token, deps.api)?;
      let asset = asset_info.with_balance(amount);
      let sender = info.sender;

      _claim(&mut msgs, &mut deps, &env, sender.clone(), &asset.info)?;

      let mut deposits = DEPOSITS.load(deps.storage, sender.clone()).unwrap_or_default();
      deposits.remove(&asset)?;
      DEPOSITS.save(deps.storage, sender.clone(), &deposits)?;

      let mut totals = TOTAL.load(deps.storage).unwrap_or_default();
      totals.remove(&asset)?;
      TOTAL.save(deps.storage, &totals)?;

      msgs.push(asset.transfer_msg(sender)?);

      attrs.push(attr("mock/amount", asset.to_string()));
    },
  }
  Ok(
    Response::new()
      .add_attribute("action", format!("mock/{0}", msg))
      .add_attributes(attrs)
      .add_messages(msgs),
  )
}

fn _claim(
  msgs: &mut Vec<CosmosMsg>,
  deps: &mut DepsMut,
  env: &Env,
  user: Addr,
  asset_info: &AssetInfo,
) -> StdResult<()> {
  let key = (user.clone(), asset_info);
  let last = LAST_CLAIM.load(deps.storage, key.clone()).unwrap_or_default();

  if last == 0 {
    LAST_CLAIM.save(deps.storage, key, &env.block.time.seconds())?;
    return Ok(());
  }

  let owned = DEPOSITS.load(deps.storage, user.clone())?.get(asset_info);
  let total = TOTAL.load(deps.storage)?.get(asset_info);
  let config = CONFIG.load(deps.storage)?;

  let seconds = env.block.time.seconds() - last;

  let per_week = if *asset_info == AssetInfo::native("xxx") {
    config.per_week_xxx
  } else {
    config.per_week
  };

  let emissions =
    per_week.multiply_ratio(Uint128::new(seconds.into()), Uint128::new(SECONDS_PER_WEEK.into()));

  LAST_CLAIM.save(deps.storage, key, &env.block.time.seconds())?;

  match (owned, total) {
    (Some(owned), Some(total)) => {
      let share = emissions.multiply_ratio(owned.amount, total.amount);
      if !share.is_zero() {
        msgs.push(config.emission.with_balance(share).transfer_msg(user).unwrap())
      }
    },
    _ => {
      // ignore
    },
  }

  Ok(())
}

pub fn determine_asset_info(maybe_asset_info: &str, api: &dyn Api) -> StdResult<AssetInfo> {
  if api.addr_validate(maybe_asset_info).is_ok() {
    Ok(AssetInfo::Cw20(Addr::unchecked(maybe_asset_info)))
  } else if validate_native_denom(maybe_asset_info).is_ok() {
    Ok(AssetInfo::Native(maybe_asset_info.to_string()))
  } else {
    Err(StdError::generic_err(format!("Cannot determine asset info from {maybe_asset_info}")))
  }
}

pub const DENOM_MAX_LENGTH: usize = 128;

pub fn validate_native_denom(denom: &str) -> StdResult<()> {
  // if denom.len() < 3 || denom.len() > DENOM_MAX_LENGTH {
  //   return Err(StdError::generic_err(format!(
  //     "Invalid denom length [3,{DENOM_MAX_LENGTH}]: {denom}"
  //   )));
  // }

  let mut chars = denom.chars();
  let first = chars.next().unwrap();
  if !first.is_ascii_alphabetic() {
    return Err(StdError::generic_err(format!("First character is not ASCII alphabetic: {denom}")));
  }

  let set = ['/', ':', '.', '_', '-'];
  for c in chars {
    if !(c.is_ascii_alphanumeric() || set.contains(&c)) {
      return Err(StdError::generic_err(format!(
        "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -: {denom}"
      )));
    }
  }

  Ok(())
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
  match msg {}
}
