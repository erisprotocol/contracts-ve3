use cosmwasm_std::{Addr, Api, StdResult};

/// Returns a lowercased, validated address upon success if present.
pub fn addr_opt_validate(api: &dyn Api, addr: &Option<String>) -> StdResult<Option<Addr>> {
  addr.as_ref().map(|addr| api.addr_validate(addr)).transpose()
}

/// Bulk validation and conversion between [`String`] -> [`Addr`] for an array of addresses.
/// If any address is invalid, the function returns [`StdError`].
pub fn validate_addresses(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
  admins.iter().map(|addr| api.addr_validate(addr)).collect()
}

pub fn addr_opt_fallback(api: &dyn Api, addr: &Option<String>, fallback: Addr) -> StdResult<Addr> {
  Ok(if let Some(addr) = addr {
    api.addr_validate(addr)?
  } else {
    fallback
  })
}
