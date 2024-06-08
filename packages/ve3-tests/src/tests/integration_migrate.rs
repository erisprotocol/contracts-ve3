use crate::common::suite::TestingSuite;

#[test]
fn test_config_default() {
  let mut suite = TestingSuite::def();
  suite.init();
  suite.migrate();
}
