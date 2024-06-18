use super::suite::TestingSuite;
use crate::mocks::eris_hub_mock::ExecuteMsg;
use cosmwasm_std::{Addr, Decimal};
use cw_multi_test::{AppResponse, Executor};

#[allow(dead_code)]
impl TestingSuite {
  // fn contract_3(&self) -> Addr {
  //   self.addresses.eris_hub_mock.clone()
  // }

  // pub fn e_hub_update_exchange_rate(
  //   &mut self,
  //   exchange_rate: Decimal,
  //   sender: &str,
  //   result: impl Fn(Result<AppResponse, anyhow::Error>),
  // ) -> &mut TestingSuite {
  //   let msg = ExecuteMsg::UpdateExchangeRate {
  //     exchange_rate,
  //   };
  //   let sender = self.address(sender);
  //   result(self.app.execute_contract(sender, self.contract_3(), &msg, &[]));
  //   self
  // }
}
