use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  entry_point, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw_asset::AssetInfo;
use cw_storage_plus::Item;
use ve3_shared::{
  error::SharedError,
  extensions::asset_info_ext::AssetInfoExt,
  helpers::{assets::Assets, general::addr_opt_fallback},
};
use ve3_zapper::error::ContractError;
pub type ContractResult = Result<Response, ContractError>;

#[cw_serde]
pub struct InstantiateMsg {
  pub exchange_rate: Vec<(AssetInfo, AssetInfo, Decimal)>,
  pub assets: Assets,
}

#[cw_serde]
pub struct Config {
  pub exchange_rate: Vec<(AssetInfo, AssetInfo, Decimal)>,
  pub assets: Assets,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[entry_point]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  CONFIG.save(
    deps.storage,
    &Config {
      exchange_rate: msg.exchange_rate,
      assets: msg.assets,
    },
  )?;
  Ok(Response::new())
}

#[entry_point]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ve3_shared::msgs_zapper::ExecuteMsg,
) -> ContractResult {
  match msg {
    ve3_shared::msgs_zapper::ExecuteMsg::Swap {
      into,
      assets,
      min_received,
      receiver,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      let asset = assets[0].clone();
      let into = into.check(deps.api, None)?;

      let exchange_rate =
        config.exchange_rate.iter().find(|a| a.0 == asset && a.1 == into).unwrap().2;
      let previously = config.assets.get(&asset).map(|a| a.amount).unwrap_or_default();
      let currently = asset.query_balance(&deps.querier, env.contract.address.to_string())?;
      let received = currently - previously;

      let returning = received * exchange_rate;
      let receiver = addr_opt_fallback(deps.api, &receiver, info.sender)?;
      let returning = into.with_balance(returning);

      if let Some(min_received) = min_received {
        if returning.amount < min_received {
          return Err(SharedError::Std(StdError::generic_err("not enough returnt")))?;
        }
      }

      config.assets.add(&asset.with_balance(received));
      config.assets.remove(&returning)?;
      CONFIG.save(deps.storage, &config)?;

      Ok(
        Response::new()
          .add_attribute("action", "zapper/callback_send_result")
          .add_attribute("amount", returning.to_string())
          .add_message(returning.transfer_msg(receiver)?),
      )
    },

    _ => todo!(),
  }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: ve3_shared::msgs_zapper::QueryMsg) -> StdResult<Binary> {
  Ok(Binary::default())
}
