use std::convert::TryInto;

use cosmwasm_std::{coin, to_json_binary, Api, Coin, CosmosMsg, MessageInfo, WasmMsg};
use cw20::{Cw20ExecuteMsg, Expiration};
use cw_asset::{Asset, AssetError, AssetInfo, AssetInfoBase, AssetInfoUnchecked};
use serde::Serialize;

use crate::error::SharedError;

pub trait AssetExt {
  fn assert_sent(&self, info: &MessageInfo) -> Result<(), SharedError>;
  fn increase_allowance_msg(
    &self,
    spender: String,
    expires: Option<Expiration>,
  ) -> Result<CosmosMsg, SharedError>;

  fn to_coin(&self) -> Result<Coin, AssetError>;
  fn send_or_execute_msg<A: Into<String>, T: Serialize + Sized>(
    &self,
    to: A,
    msg: &T,
  ) -> Result<CosmosMsg, SharedError>;
}

impl AssetExt for Asset {
  fn assert_sent(&self, info: &MessageInfo) -> Result<(), SharedError> {
    vec![self].assert_sent(info)
  }

  fn increase_allowance_msg(
    &self,
    spender: String,
    expires: Option<Expiration>,
  ) -> Result<CosmosMsg, SharedError> {
    match &self.info {
      AssetInfoBase::Cw20(addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
          spender,
          amount: self.amount,
          expires,
        })?,
        funds: vec![],
      })),
      _ => Err(SharedError::NotSupported("only cw20".to_string())),
    }
  }

  fn to_coin(&self) -> Result<Coin, AssetError> {
    self.try_into()
  }

  fn send_or_execute_msg<A: Into<String>, T: Serialize + Sized>(
    &self,
    contract: A,
    msg: &T,
  ) -> Result<CosmosMsg, SharedError> {
    match &self.info {
      AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.into(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
          contract: contract.into(),
          amount: self.amount,
          msg: to_json_binary(msg)?,
        })?,
        funds: vec![],
      })),
      AssetInfo::Native(denom) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.into(),
        msg: to_json_binary(msg)?,
        funds: vec![coin(self.amount.u128(), denom)],
      })),
      _ => Err(SharedError::NotSupported("only cw20 or native".to_string())),
    }
  }
}

pub trait AssetsExt {
  fn assert_sent(self, info: &MessageInfo) -> Result<(), SharedError>;
}

impl AssetsExt for Vec<&Asset> {
  fn assert_sent(self, info: &MessageInfo) -> Result<(), SharedError> {
    // ignore empty amounts, e.g. if fee is empty
    let relevant: Vec<_> = self.into_iter().filter(|a| !a.amount.is_zero()).collect();
    if info.funds.len() != relevant.len() {
      Err(SharedError::WrongDeposit(format!("expected {0} coins", relevant.len())))
    } else {
      for asset in relevant {
        let coin: Coin = asset.try_into()?;
        if !info.funds.contains(&coin) {
          return Err(SharedError::WrongDeposit(format!("expected {0} coins", coin)));
        }
      }
      Ok(())
    }
  }
}

pub trait AssetsUncheckedExt {
  fn check(self, api: &dyn Api) -> Result<Vec<AssetInfo>, AssetError>;
}

impl AssetsUncheckedExt for Vec<AssetInfoUnchecked> {
  fn check(self, api: &dyn Api) -> Result<Vec<AssetInfo>, AssetError> {
    self.into_iter().map(|a| a.check(api, None)).collect::<Result<Vec<AssetInfo>, AssetError>>()
  }
}
