use std::collections::HashSet;

use super::suite::TestingSuite;
use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::msgs_connector_alliance::*;

#[allow(dead_code)]
impl TestingSuite {
  fn contract_5(&self) -> Addr {
    self.addresses.ve3_connector_alliance_1.clone()
  }

  pub fn e_alliance_claim_rewards(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRewards {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_5(), &msg, &[]));
    self
  }

  pub fn e_alliance_alliance_delegate(
    &mut self,
    alliance_delegate_msg: AllianceDelegateMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AllianceDelegate(alliance_delegate_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_5(), &msg, &[]));
    self
  }

  pub fn e_alliance_alliance_undelegate(
    &mut self,
    alliance_undelegate_msg: AllianceUndelegateMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AllianceUndelegate(alliance_undelegate_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_5(), &msg, &[]));
    self
  }

  pub fn e_alliance_alliance_redelegate(
    &mut self,
    alliance_redelegate_msg: AllianceRedelegateMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AllianceRedelegate(alliance_redelegate_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_5(), &msg, &[]));
    self
  }

  pub fn e_alliance_remove_validator(
    &mut self,
    validator: String,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::RemoveValidator {
      validator,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_5(), &msg, &[]));
    self
  }

  pub fn e_alliance_callback(
    &mut self,
    callback_msg: CallbackMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Callback(callback_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_5(), &msg, &[]));
    self
  }

  pub fn q_alliance_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_5(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_alliance_validators(&mut self, result: impl Fn(StdResult<HashSet<Addr>>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_5(), &QueryMsg::Validators {});
    result(response);
    self
  }
}