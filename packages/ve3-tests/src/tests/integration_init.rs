use cosmwasm_std::coin;

use crate::common::suite::TestingSuite;

#[test]
fn test_init() {
  let mut suite =
    TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uluna".to_string())]);

  suite.instantiate_default();
}
