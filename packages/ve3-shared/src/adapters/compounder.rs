use crate::{
  error::SharedError,
  msgs_asset_compounding::{Cw20HookMsg, ExecuteMsg},
};
use cosmwasm_std::{coins, to_json_binary, Addr, CosmosMsg, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_asset::Asset;

pub struct Compounder(pub Addr);

impl Compounder {
  pub fn deposit_msg(
    &self,
    asset: Asset,
    gauge: String,
    recipient: Option<String>,
  ) -> Result<CosmosMsg, SharedError> {
    match asset.info {
      cw_asset::AssetInfoBase::Native(native) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: self.0.to_string(),
        msg: to_json_binary(&ExecuteMsg::Stake {
          recipient,
          gauge,
        })?,
        funds: coins(asset.amount.u128(), native),
      })),
      cw_asset::AssetInfoBase::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
          contract: self.0.to_string(),
          amount: asset.amount,
          msg: to_json_binary(&Cw20HookMsg::Stake {
            recipient,
            gauge,
          })?,
        })?,
      })),
      _ => Err(SharedError::NotSupported("asset type".to_string())),
    }
  }
}
