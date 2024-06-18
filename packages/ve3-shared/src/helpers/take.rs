use cosmwasm_std::Uint128;

pub fn compute_share_amount(
  shares: Uint128,
  balance_amount: Uint128,
  asset_available: Uint128,
) -> Uint128 {
  if asset_available.is_zero() {
    balance_amount
  } else if shares == asset_available {
    return balance_amount;
  } else {
    balance_amount.multiply_ratio(shares, asset_available)
  }
}

pub fn compute_balance_amount(
  shares: Uint128,
  share_amount: Uint128,
  asset_available: Uint128,
) -> Uint128 {
  if shares.is_zero() {
    Uint128::zero()
  } else if shares == asset_available {
    return share_amount;
  } else {
    share_amount.multiply_ratio(asset_available, shares)
  }
}
