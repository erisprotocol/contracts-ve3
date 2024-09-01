use cosmwasm_std::{Addr, Uint128};
use cw_asset::{Asset, AssetInfo};

pub fn native<A: Into<String>, B: Into<Uint128>>(d: A, a: B) -> Asset {
  Asset::native(d, a)
}

pub fn native_info(d: &str) -> AssetInfo {
  AssetInfo::native(d)
}
pub fn cw20_info(d: &str) -> AssetInfo {
  AssetInfo::cw20(Addr::unchecked(d))
}

pub fn uluna(amount: u32) -> Asset {
  native("uluna", u(amount))
}

#[allow(non_snake_case)]
pub fn Native(denom: &str) -> AssetInfo {
  cw_asset::AssetInfoBase::Native(denom.to_string())
}
#[allow(non_snake_case)]
pub fn Addr(denom: &str) -> Addr {
  Addr::unchecked(denom.to_string())
}
#[allow(non_snake_case)]
pub fn Cw20(denom: Addr) -> AssetInfo {
  cw_asset::AssetInfoBase::Cw20(denom)
}

pub fn cw20<A: Into<Addr>, B: Into<Uint128>>(addr: A, a: B) -> Asset {
  Asset::cw20(addr, a)
}

pub fn u(a: u32) -> Uint128 {
  Uint128::new(a.into())
}

#[allow(non_snake_case)]
pub fn Uint128(a: u32) -> Uint128 {
  Uint128::new(a.into())
}

#[allow(dead_code)]
trait U {
  fn u(self) -> Uint128;
}

impl U for i32 {
  fn u(self) -> Uint128 {
    Uint128::new((self as u32).into())
  }
}

impl U for u32 {
  fn u(self) -> Uint128 {
    Uint128::new(self.into())
  }
}
