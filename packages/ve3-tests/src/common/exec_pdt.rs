use std::collections::HashSet;

use cosmwasm_std::{Addr, StdResult, Uint128};
use cw_asset::{Asset, AssetInfoBase, AssetInfoUnchecked};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{extensions::asset_ext::AssetExt, msgs_phoenix_treasury::*};

use super::suite::TestingSuite;

impl TestingSuite {
  fn contract_pdt(&self) -> Addr {
    self.addresses.pdt.clone()
  }

  pub fn e_pdt_update_veto_config(
    &mut self,
    vetos: Vec<VetoRight<String>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateVetoConfig {
      vetos,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_update_config(
    &mut self,
    add_oracle: Option<Vec<(AssetInfoBase<String>, Oracle<String>)>>,
    remove_oracle: Option<Vec<AssetInfoBase<String>>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateConfig {
      add_oracle,
      remove_oracle,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_setup(
    &mut self,
    name: &str,
    action: TreasuryActionSetup,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Setup {
      name: name.to_string(),
      action,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_clawback(
    &mut self,
    recipient: &str,
    assets: Vec<AssetInfoUnchecked>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::Clawback {
      assets,
      recipient: self.address(recipient).to_string(),
    };

    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_cancel(
    &mut self,
    id: u64,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Cancel {
      id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_veto(
    &mut self,
    id: u64,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Veto {
      id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_claim(
    &mut self,
    id: u64,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Claim {
      id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_update_milestone(
    &mut self,
    id: u64,
    index: u64,
    enabled: bool,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateMilestone {
      id,
      index,
      enabled,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_execute_dca(
    &mut self,
    id: u64,
    min_received: Option<Uint128>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ExecuteDca {
      id,
      min_received,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_claim_rewards(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRewards {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_alliance_delegate(
    &mut self,
    delegate_msg: AllianceDelegateMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AllianceDelegate(delegate_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_execute_otc_no_coins(
    &mut self,
    id: u64,
    amount: Uint128,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::ExecuteOtc {
      id,
      offer_amount: amount,
    };

    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_execute_otc(
    &mut self,
    id: u64,
    asset: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::ExecuteOtc {
      id,
      offer_amount: asset.amount,
    };

    let sender = self.address(sender);
    result(self.app.execute_contract(
      sender,
      self.contract_pdt(),
      &msg,
      &[asset.to_coin().unwrap()],
    ));
    self
  }

  pub fn e_pdt_alliance_undelegate(
    &mut self,
    undelegate_msg: AllianceUndelegateMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AllianceUndelegate(undelegate_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_alliance_redelegate(
    &mut self,
    redelegate_msg: AllianceRedelegateMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AllianceRedelegate(redelegate_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn e_pdt_remove_validator(
    &mut self,
    validator: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::RemoveValidator {
      validator: validator.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_pdt(), &msg, &[]));
    self
  }

  pub fn q_pdt_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_pdt(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_pdt_state(&mut self, result: impl Fn(StdResult<State>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_pdt(), &QueryMsg::State {});
    result(response);
    self
  }

  pub fn q_pdt_validators(&mut self, result: impl Fn(StdResult<HashSet<Addr>>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_pdt(), &QueryMsg::Validators {});
    result(response);
    self
  }

  pub fn q_pdt_actions(
    &mut self,
    start_after: Option<u64>,
    limit: Option<u32>,
    result: impl Fn(StdResult<Vec<TreasuryAction>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_pdt(),
      &QueryMsg::Actions {
        start_after,
        limit,
        direction: None,
      },
    );
    result(response);
    self
  }

  pub fn q_pdt_actions_direction(
    &mut self,
    start_after: Option<u64>,
    limit: Option<u32>,
    direction: Direction,
    result: impl Fn(StdResult<Vec<TreasuryAction>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_pdt(),
      &QueryMsg::Actions {
        start_after,
        limit,
        direction: Some(direction),
      },
    );
    result(response);
    self
  }

  pub fn q_pdt_user_actions(
    &mut self,
    user: &str,
    start_after: Option<u64>,
    limit: Option<u32>,
    result: impl Fn(StdResult<Vec<TreasuryAction>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_pdt(),
      &QueryMsg::UserActions {
        user: self.address(user).to_string(),
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub fn q_pdt_action(&mut self, id: u64, result: impl Fn(StdResult<TreasuryAction>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_pdt(),
      &QueryMsg::Action {
        id,
      },
    );
    result(response);
    self
  }

  pub fn q_pdt_balances(
    &mut self,
    assets: Option<Vec<AssetInfoUnchecked>>,
    result: impl Fn(StdResult<BalancesResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_pdt(),
      &QueryMsg::Balances {
        assets,
      },
    );
    result(response);
    self
  }

  pub fn q_pdt_oracle_prices(
    &mut self,
    assets: Option<Vec<AssetInfoUnchecked>>,
    result: impl Fn(StdResult<OraclesResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_pdt(),
      &QueryMsg::OraclePrices {
        assets,
      },
    );
    result(response);
    self
  }
}
