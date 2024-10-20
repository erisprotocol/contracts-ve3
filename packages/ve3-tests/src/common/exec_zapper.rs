use cosmwasm_std::{Addr, StdResult, Uint128};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::msgs_zapper::*;

use super::suite::TestingSuite;

impl TestingSuite {
  fn contract_zapper(&self) -> Addr {
    self.addresses.ve3_zapper.clone()
  }
  pub fn e_zapper_zap(
      &mut self,
      into: AssetInfoUnchecked,
      assets: Vec<AssetInfo>,
      min_received: Option<Uint128>,
      post_action: Option<PostActionCreate>,
      sender: &str,
      result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
      let msg = ExecuteMsg::Zap {
          into,
          assets,
          min_received,
          post_action,
      };

      let sender = self.address(sender);
      result(self.app.execute_contract(sender, self.contract_zapper(), &msg, &[]));
      self
  }

  pub fn e_zapper_create_lp(
    &mut self,
    stage: StageType,
    assets: Vec<AssetInfo>,
    min_received: Option<Uint128>,
    post_action: Option<PostActionCreate>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::CreateLp {
      stage,
      assets,
      min_received,
      post_action,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_zapper(), &msg, &[]));
    self
  }

  pub fn e_zapper_withdraw_lp(
    &mut self,
    stage: StageType,
    min_received: Option<Vec<Asset>>,
    post_action: Option<PostActionWithdraw>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::WithdrawLp {
      stage,
      min_received,
      post_action,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_zapper(), &msg, &[]));
    self
  }

  pub fn e_zapper_swap(
    &mut self,
    into: AssetInfoUnchecked,
    assets: Vec<AssetInfo>,
    min_received: Option<Uint128>,
    receiver: Option<&str>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Swap {
      into,
      assets,
      min_received,
      receiver: receiver.map(|r| r.to_string()),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_zapper(), &msg, &[]));
    self
  }

  pub fn e_zapper_update_config(
    &mut self,
    insert_routes: Option<Vec<RouteInit>>,
    delete_routes: Option<Vec<RouteDelete>>,
    update_centers: Option<Vec<AssetInfoUnchecked>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateConfig {
      insert_routes,
      delete_routes,
      update_centers,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_zapper(), &msg, &[]));
    self
  }

  pub fn e_zapper_callback(
    &mut self,
    callback_msg: CallbackMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Callback(callback_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_zapper(), &msg, &[]));
    self
  }

  pub fn q_zapper_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_zapper(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_zapper_get_routes(
    &mut self,
    start_after: Option<(AssetInfo, AssetInfo)>,
    limit: Option<u32>,
    result: impl Fn(StdResult<Vec<RouteResponseItem>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_zapper(),
      &QueryMsg::GetRoutes {
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub fn q_zapper_get_route(
    &mut self,
    from: AssetInfo,
    to: AssetInfo,
    result: impl Fn(StdResult<RouteResponseItem>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_zapper(),
      &QueryMsg::GetRoute {
        from,
        to,
      },
    );
    result(response);
    self
  }

  pub fn q_zapper_supports_swap(
    &mut self,
    from: AssetInfo,
    to: AssetInfo,
    result: impl Fn(StdResult<SupportsSwapResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_zapper(),
      &QueryMsg::SupportsSwap {
        from,
        to,
      },
    );
    result(response);
    self
  }
}
