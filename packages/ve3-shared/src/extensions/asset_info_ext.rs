use cosmwasm_std::{
  Addr, BankQuery, MessageInfo, QuerierWrapper, QueryRequest, StdResult, SupplyResponse, Uint128,
};
use cw20::{Cw20QueryMsg, TokenInfoResponse};
use cw_asset::{Asset, AssetError, AssetInfo};

use crate::error::SharedError;

pub trait AssetInfoExt {
  /// simplifies converting an AssetInfo to an Asset with balance
  fn with_balance(&self, balance: Uint128) -> Asset;
  fn with_balance_u128(&self, amount: u128) -> Asset;
  fn with_balance_query(
    &self,
    querier: &QuerierWrapper,
    address: &Addr,
  ) -> Result<Asset, AssetError>;
  fn assert_native(&self) -> Result<(), SharedError>;
  fn is_native(&self) -> bool;

  fn assert_received(&self, info: &MessageInfo) -> Result<Asset, SharedError>;

  fn total_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128>;
}

impl AssetInfoExt for AssetInfo {
  fn assert_native(&self) -> Result<(), SharedError> {
    match self {
      cw_asset::AssetInfoBase::Native(_) => Ok(()),
      _ => Err(SharedError::NotSupported("must be native".to_string())),
    }
  }

  fn is_native(&self) -> bool {
    matches!(self, cw_asset::AssetInfoBase::Native(_))
  }

  fn with_balance(&self, amount: Uint128) -> Asset {
    match self {
      cw_asset::AssetInfoBase::Native(denom) => Asset::native(denom, amount),
      cw_asset::AssetInfoBase::Cw20(contract_addr) => Asset::cw20(contract_addr.clone(), amount),
      _ => todo!(),
    }
  }

  fn with_balance_u128(&self, amount: u128) -> Asset {
    self.with_balance(Uint128::new(amount))
  }

  fn with_balance_query(
    &self,
    querier: &QuerierWrapper,
    address: &Addr,
  ) -> Result<Asset, AssetError> {
    let balance = self.query_balance(querier, address.clone())?;
    Ok(self.with_balance(balance))
  }

  fn total_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
    match self {
      cw_asset::AssetInfoBase::Native(denom) => {
        let response: SupplyResponse = querier.query(&QueryRequest::Bank(BankQuery::Supply {
          denom: denom.to_string(),
        }))?;

        Ok(response.amount.amount)
      },
      cw_asset::AssetInfoBase::Cw20(token_addr) => {
        let token_info: TokenInfoResponse =
          querier.query_wasm_smart(token_addr, &Cw20QueryMsg::TokenInfo {})?;
        Ok(token_info.total_supply)
      },
      _ => todo!(),
    }
  }

  fn assert_received(&self, info: &MessageInfo) -> Result<Asset, SharedError> {
    if info.funds.is_empty() {
      return Err(SharedError::WrongDeposit("no asset sent".to_string()));
    }

    if info.funds.len() > 1 {
      return Err(SharedError::WrongDeposit("too many assets sent".to_string()));
    }

    let fund = &info.funds[0];
    let info = AssetInfo::native(fund.denom.clone());
    let is_allowed = *self == info;

    if !is_allowed {
      return Err(SharedError::WrongDeposit(format!("wrong deposit {0}", fund.denom)));
    }

    if fund.amount.is_zero() {
      return Err(SharedError::WrongDeposit("requires amount".to_string()));
    }

    Ok(info.with_balance(fund.amount))
  }
}
