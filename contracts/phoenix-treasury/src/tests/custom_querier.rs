use super::cw20_querier::Cw20Querier;
use cosmwasm_std::testing::{BankQuerier, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
  from_json, to_json_binary, Addr, Binary, Coin, ContractResult, Decimal, Empty, Querier,
  QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use std::collections::HashMap;
use std::ops::Deref;
use ve3_shared::adapters::global_config_adapter::ADDRESSES;
use ve3_shared::constants::{at_asset_staking, AT_DELEGATION_CONTROLLER};

#[derive(Default)]
pub(super) struct CustomQuerier {
  pub bank_querier: BankQuerier,
  pub cw20_querier: Cw20Querier,
}

impl Querier for CustomQuerier {
  fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
    let request: QueryRequest<_> = match from_json(bin_request) {
      Ok(v) => v,
      Err(e) => {
        return Err(SystemError::InvalidRequest {
          error: format!("Parsing query request: {}", e),
          request: bin_request.into(),
        })
        .into()
      },
    };
    self.handle_query(&request)
  }
}

fn right(map: &mut HashMap<Binary, Addr>, right: &str, addr: &str) {
  map.insert(ADDRESSES.key(right.to_string()).deref().into(), Addr::unchecked(addr.to_string()));
}

impl CustomQuerier {
  pub fn set_bank_balances(&mut self, balances: &[Coin]) {
    self.bank_querier = BankQuerier::new(&[(MOCK_CONTRACT_ADDR, balances)])
  }

  #[allow(dead_code)]
  pub fn set_cw20_balance(&mut self, token: &str, user: &str, balance: u64) {
    match self.cw20_querier.balances.get_mut(token) {
      Some(contract_balances) => {
        contract_balances.insert(user.to_string(), balance.into());
      },
      None => {
        let mut contract_balances: HashMap<String, u128> = HashMap::default();
        contract_balances.insert(user.to_string(), balance.into());
        self.cw20_querier.balances.insert(token.to_string(), contract_balances);
      },
    };
  }

  #[allow(dead_code)]
  pub fn get_cw20_balance(&mut self, token: &str, user: &str) -> u128 {
    match self.cw20_querier.balances.get_mut(token) {
      Some(contract_balances) => *contract_balances.get(&user.to_string()).unwrap_or(&0u128),
      None => 0u128,
    }
  }

  #[allow(dead_code)]
  pub fn set_cw20_total_supply(&mut self, token: &str, total_supply: u128) {
    self.cw20_querier.total_supplies.insert(token.to_string(), total_supply);
  }

  #[allow(dead_code)]
  pub fn get_cw20_total_supply(&mut self, token: &str) -> u128 {
    *self.cw20_querier.total_supplies.get(token).unwrap_or(&0u128)
  }

  pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
    match request {
      QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr,
        key,
      }) => {
        if contract_addr == "global_config" {
          let mut allowed = HashMap::new();
          right(&mut allowed, AT_DELEGATION_CONTROLLER, "controller");
          right(&mut allowed, &at_asset_staking("test"), "lp_staking");

          match allowed.get(key) {
            Some(result) => SystemResult::Ok(ContractResult::Ok(to_json_binary(result).unwrap())),
            None => SystemResult::Ok(ContractResult::Ok(Binary(vec![]))),
          }
        } else {
          SystemResult::Err(SystemError::InvalidRequest {
            error: format!("[mock] unsupported query: {:?}", "needs to be global_config"),
            request: Default::default(),
          })
        }
      },

      QueryRequest::Wasm(WasmQuery::Smart {
        msg,
        contract_addr,
      }) => {
        if let Ok(query) = from_json::<cw20::Cw20QueryMsg>(msg) {
          return self.cw20_querier.handle_query(contract_addr, query);
        }

        if let Ok(query) = from_json::<ve3_shared::adapters::eris::QueryMsg>(msg) {
          return match query {
            ve3_shared::adapters::eris::QueryMsg::State {} => SystemResult::Ok(ContractResult::Ok(
              to_json_binary(&ve3_shared::adapters::eris::StateResponse {
                exchange_rate: Decimal::from_ratio(12u128, 10u128),
              })
              .unwrap(),
            )),
          };
        }

        err_unsupported_query(msg, Some(contract_addr.to_string()))
      },

      QueryRequest::Bank(query) => self.bank_querier.query(query),

      _ => err_unsupported_query(request, None),
    }
  }
}

pub(super) fn err_unsupported_query<T: std::fmt::Debug>(
  request: T,
  contract: Option<String>,
) -> QuerierResult {
  SystemResult::Err(SystemError::InvalidRequest {
    error: format!("[mock] unsupported query: {contract:?}: {:?}", request),
    request: Default::default(),
  })
}
