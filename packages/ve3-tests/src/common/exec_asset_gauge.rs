use cosmwasm_std::{to_json_binary, Addr, StdResult};
use cw_asset::Asset;
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{
  extensions::asset_ext::AssetExt,
  helpers::time::{Time, Times},
  msgs_asset_gauge::*,
  msgs_voting_escrow::LockInfoResponse,
};

use super::{helpers::u, suite::TestingSuite};

impl TestingSuite {
  fn contract_1(&self) -> Addr {
    self.addresses.ve3_asset_gauge.clone()
  }

  pub fn e_gauge_vote(
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

  pub fn e_gauge_update_vote(
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

  pub fn e_gauge_set_distribution(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::SetDistribution {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  pub fn e_gauge_claim_rebase(
    &mut self,
    token_id: Option<&str>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRebase {
      token_id: token_id.map(|id| id.to_string()),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
    self
  }

  #[track_caller]
  pub fn e_gauge_add_rebase_in_ampluna(
    &mut self,
    amount: u128,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AddRebase {};
    let sender = self.address("creator");

    // let res = self.app.execute_contract(
    //   sender.clone(),
    //   self.addresses.eris_hub.clone(),
    //   &eris::hub::ExecuteMsg::Bond {
    //     receiver: None,
    //   },
    //   &[asset.to_coin().unwrap()],
    // );
    // res.assert_valid();

    let ampluna = self.addresses.eris_hub_cw20_ampluna.clone();

    // let balance: cw20::BalanceResponse = self
    //   .app
    //   .wrap()
    //   .query_wasm_smart(
    //     ampluna.clone(),
    //     &cw20::Cw20QueryMsg::Balance {
    //       address: sender.to_string(),
    //     },
    //   )
    //   .unwrap();

    let send_msg = cw20_base::msg::ExecuteMsg::Send {
      contract: self.contract_1().to_string(),
      amount: u(amount),
      msg: to_json_binary(&msg).unwrap(),
    };

    result(self.app.execute_contract(sender, ampluna.clone(), &send_msg, &[]));

    self
  }

  pub fn e_gauge_add_rebase(
    &mut self,
    sender: &str,
    asset: Asset,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AddRebase {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_1(), &msg, &[asset.to_coin().unwrap()]));
    self
  }
  // pub fn e_gauge_clear_gauge_state(
  //   &mut self,
  //   gauge: String,
  //   limit: Option<usize>,
  //   sender: &str,
  //   result: impl Fn(Result<AppResponse, anyhow::Error>),
  // ) -> &mut TestingSuite {
  //   let msg = ExecuteMsg::ClearGaugeState {
  //     gauge,
  //     limit,
  //   };
  //   let sender = self.address(sender);
  //   result(self.app.execute_contract(sender, self.contract_1(), &msg, &[]));
  //   self
  // }

  pub fn e_gauge_update_config(
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

  pub fn q_gauge_user_info(
    &mut self,
    user: &str,
    time: Option<Time>,
    result: impl Fn(StdResult<UserInfoExtendedResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserInfo {
        user: self.address(user).to_string(),
        time,
      },
    );
    result(response);
    self
  }

  pub fn q_gauge_user_infos(
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

  pub fn q_gauge_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_1(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_gauge_user_shares(
    &mut self,
    user: &str,
    times: Option<Times>,
    result: impl Fn(StdResult<UserSharesResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserShares {
        user: self.address(user),
        times,
      },
    );
    result(response);
    self
  }

  pub fn q_gauge_user_first_participation(
    &mut self,
    user: &str,
    result: impl Fn(StdResult<UserFirstParticipationResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserFirstParticipation {
        user: self.address(user),
      },
    );
    result(response);
    self
  }

  pub fn q_gauge_gauge_info(
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

  pub fn q_gauge_gauge_infos(
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

  pub fn q_gauge_distribution(
    &mut self,
    gauge: String,
    time: Option<Time>,
    result: impl Fn(StdResult<GaugeDistributionResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::Distribution {
        gauge,
        time,
      },
    );
    result(response);
    self
  }

  pub fn q_gauge_distributions(
    &mut self,
    time: Option<Time>,
    result: impl Fn(StdResult<Vec<GaugeDistributionResponse>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::Distributions {
        time,
      },
    );
    result(response);
    self
  }

  pub fn q_gauge_user_pending_rebase(
    &mut self,
    user: &str,
    result: impl Fn(StdResult<UserPendingRebaseResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_1(),
      &QueryMsg::UserPendingRebase {
        user: self.address(user),
      },
    );
    result(response);
    self
  }
}
