use crate::common::suite::TestingSuite;

#[test]
fn test_zapper() {
  let mut suite = TestingSuite::def();

  let user1 = suite.address("user1").to_string();
  let addr = suite.addresses.clone();

  suite.init();
}
