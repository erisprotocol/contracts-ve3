use cosmwasm_std::Decimal;

#[track_caller]
#[allow(dead_code)]
pub fn assert_close(actual: Decimal, expected: Decimal, delta: Decimal) {
  assert!(actual > expected.saturating_sub(delta));
  let err = format!("{actual} < {expected} + {delta}");
  assert!(actual < expected + delta, "{}", err);
}
