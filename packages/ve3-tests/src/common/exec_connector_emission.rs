use cosmwasm_std::{Addr, Decimal, StdResult, Uint128};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::msgs_connector_emission::*;

use super::suite::TestingSuite;

impl TestingSuite {
  fn contract_emissions(&self) -> Addr {
    self.addresses.ve3_connector_emissions.clone()
  }

  pub fn e_emission_claim_rewards(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRewards {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_emissions(), &msg, &[]));
    self
  }

  #[allow(clippy::too_many_arguments)]
  pub fn e_emission_update_config(
    &mut self,
    emissions_per_s: Option<Uint128>,
    team_share: Option<Decimal>,
    rebase_config: Option<ve3_shared::msgs_connector_emission::RebaseConfg>,
    mint_config: Option<MintConfig>,
    enabled: Option<bool>,
    gauge: Option<&str>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateConfig {
      emissions_per_s,
      team_share,
      rebase_config,
      mint_config,
      enabled,
      gauge: gauge.map(|g| g.to_string()),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_emissions(), &msg, &[]));
    self
  }

  pub fn q_emission_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_emissions(), &QueryMsg::Config {});
    result(response);
    self
  }
}
