use cosmwasm_std::{Addr, Uint128};
use cw_asset::Asset;

pub fn native<A: Into<String>, B: Into<Uint128>>(d: A, a: B) -> Asset {
  Asset::native(d, a)
}

pub fn cw20<A: Into<Addr>, B: Into<Uint128>>(addr: A, a: B) -> Asset {
  Asset::cw20(addr, a)
}

pub fn u(a: u32) -> Uint128 {
  Uint128::new(a.into())
}

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
