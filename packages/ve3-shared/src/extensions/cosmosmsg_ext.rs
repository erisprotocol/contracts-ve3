use cosmwasm_std::{StdError, StdResult};

pub trait CosmosMsgExt {
  fn to_specific<T>(self) -> StdResult<cosmwasm_std::CosmosMsg<T>>;
}

impl CosmosMsgExt for cosmwasm_std::CosmosMsg {
  fn to_specific<T>(self) -> StdResult<cosmwasm_std::CosmosMsg<T>> {
    match self {
      cosmwasm_std::CosmosMsg::Bank(msg) => Ok(cosmwasm_std::CosmosMsg::Bank(msg)),
      cosmwasm_std::CosmosMsg::Wasm(msg) => Ok(cosmwasm_std::CosmosMsg::Wasm(msg)),
      // cosmwasm_std::CosmosMsg::Staking(msg) => Ok(cosmwasm_std::CosmosMsg::Staking(msg)),
      // cosmwasm_std::CosmosMsg::Distribution(msg) => {
      //     Ok(cosmwasm_std::CosmosMsg::Distribution(msg))
      // },
      cosmwasm_std::CosmosMsg::Ibc(msg) => Ok(cosmwasm_std::CosmosMsg::Ibc(msg)),
      cosmwasm_std::CosmosMsg::Gov(msg) => Ok(cosmwasm_std::CosmosMsg::Gov(msg)),
      _ => Err(StdError::generic_err("not supported")),
    }
  }
}
