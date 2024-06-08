use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{AppResponse, Executor};
use cw_ownable::{Action, Ownership};
use ve3_shared::msgs_global_config::{AddressListResponse, AddressResponse, ExecuteMsg, QueryMsg};

use super::suite::TestingSuite;

impl TestingSuite {
  fn contract_2(&self) -> Addr {
    self.addresses.ve3_global_config.clone()
  }

  pub fn e_gc_update_ownership(
    &mut self,
    action: Action,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateOwnership(action);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_2(), &msg, &[]));
    self
  }

  pub fn e_gc_set_addresses(
    &mut self,
    addresses: Vec<(String, String)>,
    lists: Vec<(String, Vec<String>)>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::SetAddresses {
      addresses,
      lists,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_2(), &msg, &[]));
    self
  }

  pub fn e_gc_clear_addresses(
    &mut self,
    addresses: Vec<String>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClearAddresses {
      addresses,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_2(), &msg, &[]));
    self
  }

  pub fn e_gc_clear_lists(
    &mut self,
    lists: Vec<String>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClearLists {
      lists,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_2(), &msg, &[]));
    self
  }

  pub fn q_gc_ownership(&mut self, result: impl Fn(StdResult<Ownership<String>>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_2(), &QueryMsg::Ownership {});
    result(response);
    self
  }

  pub fn q_gc_address(
    &mut self,
    address: String,
    result: impl Fn(StdResult<AddressResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_2(), &QueryMsg::Address(address));
    result(response);
    self
  }

  pub fn q_gc_addresses(
    &mut self,
    addresses: Vec<String>,
    result: impl Fn(StdResult<Vec<AddressResponse>>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_2(), &QueryMsg::Addresses(addresses));
    result(response);
    self
  }

  pub fn q_gc_all_addresses(
    &mut self,
    start_after: Option<String>,
    limit: Option<u32>,
    result: impl Fn(StdResult<Vec<AddressResponse>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_2(),
      &QueryMsg::AllAddresses {
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub fn q_gc_address_list(
    &mut self,
    address: String,
    result: impl Fn(StdResult<AddressListResponse>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_2(), &QueryMsg::AddressList(address));
    result(response);
    self
  }
}
