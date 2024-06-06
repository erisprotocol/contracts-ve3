use std::convert::TryInto;

use cosmwasm_std::{coin, Coin, MessageInfo};
use cw_asset::Asset;

use crate::error::SharedError;

pub trait AssetExt {
  fn assert_sent(&self, info: &MessageInfo) -> Result<(), SharedError>;
}

impl AssetExt for Asset {
  fn assert_sent(&self, info: &MessageInfo) -> Result<(), SharedError> {
    if info.funds.len() != 1 {
      Err(SharedError::WrongDeposit("expect only one coin".to_string()))
    } else if Asset::from(info.funds[0].clone()) == *self {
      Ok(())
    } else {
      Err(SharedError::WrongDeposit("assets do not match".to_string()))
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
          return Err(SharedError::WrongDeposit(format!("missing {0}", coin)));
        }
      }
      Ok(())
    }
  }
}
