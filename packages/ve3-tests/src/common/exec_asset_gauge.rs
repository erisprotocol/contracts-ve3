use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{
  helpers::time::{Time, Times},
  msgs_asset_gauge::*,
  msgs_voting_escrow::LockInfoResponse,
};

use super::suite::TestingSuite;

impl TestingSuite {
  fn contract_1(&self) -> Addr {
    self.addresses.ve3_asset_gauge.clone()
  }

  #[track_caller]
  pub fn gauge_execute(
    &mut self,
    execute: ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(creator, self.contract_1(), &execute, &[]));
    self
  }

  pub fn gauge_vote_execute(
    &mut self,
    gauge: String,
    votes: Vec<(String, u16)>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Vote {
      gauge,
      votes,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  pub fn gauge_update_vote_execute(
    &mut self,
    token_id: String,
    lock_info: LockInfoResponse,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateVote {
      token_id,
      lock_info,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  pub fn gauge_set_distribution_execute(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::SetDistribution {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  pub fn gauge_clear_gauge_state_execute(
    &mut self,
    gauge: String,
    limit: Option<usize>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClearGaugeState {
      gauge,
      limit,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  pub fn gauge_update_config_execute(
    &mut self,
    update_gauge: Option<GaugeConfig>,
    remove_gauge: Option<String>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateConfig {
      update_gauge,
      remove_gauge,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  pub fn query_gauge_user_info(
    &mut self,
    user: String,
    time: Option<Time>,
    result: impl Fn(StdResult<UserInfoExtendedResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserInfo {
        user,
        time,
      },
    );
    result(response);
    self
  }

  pub fn query_gauge_user_infos(
    &mut self,
    start_after: Option<String>,
    limit: Option<u32>,
    time: Option<Time>,
    result: impl Fn(StdResult<UserInfosResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserInfos {
        start_after,
        limit,
        time,
      },
    );
    result(response);
    self
  }

  pub fn query_gauge_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_1(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn query_gauge_user_shares(
    &mut self,
    user: Addr,
    times: Option<Times>,
    result: impl Fn(StdResult<UserSharesResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserShares {
        user,
        times,
      },
    );
    result(response);
    self
  }

  pub fn query_gauge_user_first_participation(
    &mut self,
    user: Addr,
    result: impl Fn(StdResult<UserFirstParticipationResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserFirstParticipation {
        user,
      },
    );
    result(response);
    self
  }

  pub fn query_gauge_gauge_info(
    &mut self,
    gauge: String,
    key: String,
    time: Option<Time>,
    result: impl Fn(StdResult<VotedInfoResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::GaugeInfo {
        gauge,
        key,
        time,
      },
    );
    result(response);
    self
  }

  pub fn query_gauge_gauge_infos(
    &mut self,
    gauge: String,
    keys: Option<Vec<String>>,
    time: Option<Time>,
    result: impl Fn(StdResult<GaugeInfosResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::GaugeInfos {
        gauge,
        keys,
        time,
      },
    );
    result(response);
    self
  }
}
