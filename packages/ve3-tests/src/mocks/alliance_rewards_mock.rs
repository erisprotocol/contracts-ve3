use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw_asset::AssetInfo;
use cw_storage_plus::Item;
use ve3_shared::{error::SharedError, extensions::asset_info_ext::AssetInfoExt};
pub type ContractResult = Result<Response, SharedError>;

#[cw_serde]
pub enum ExecuteMsg {
  ClaimRewards {},
}

#[cw_serde]
pub struct InstantiateMsg {
  pub reward_denom: String,
}

#[cw_serde]
pub enum QueryMsg {}

#[cw_serde]
pub struct Config {
  pub reward_denom: String,
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
      reward_denom: msg.reward_denom,
    },
  )?;
  Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::ClaimRewards {} => {
      let config = CONFIG.load(deps.storage)?;
      let transfer_msg = AssetInfo::native(config.reward_denom)
        .with_balance_query(&deps.querier, &_env.contract.address)?
        .transfer_msg(info.sender)?;

      Ok(Response::new().add_message(transfer_msg))
    },
  }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
  Err(StdError::generic_err("not supported"))
}
