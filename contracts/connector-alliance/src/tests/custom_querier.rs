use std::collections::HashMap;
use std::ops::Deref;

use cosmwasm_std::testing::{BankQuerier, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, Coin, ContractResult, Empty, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, WasmQuery,
};
use ve3_global_config::global_config_adapter::ADDRESSES;
use ve3_shared::constants::{AT_DELEGATION_CONTROLLER, AT_LP_STAKING};

#[derive(Default)]
pub(super) struct CustomQuerier {
    pub bank_querier: BankQuerier,
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

    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Raw {
                contract_addr,
                key,
            }) => {
                if contract_addr == "global_config" {
                    let mut allowed = HashMap::new();
                    right(&mut allowed, AT_DELEGATION_CONTROLLER, "controller");
                    right(&mut allowed, AT_LP_STAKING, "lp_staking");

                    match allowed.get(key) {
                        Some(result) => {
                            SystemResult::Ok(ContractResult::Ok(to_json_binary(result).unwrap()))
                        },
                        None => SystemResult::Ok(ContractResult::Ok(Binary(vec![]))),
                    }
                } else {
                    SystemResult::Err(SystemError::InvalidRequest {
                        error: format!(
                            "[mock] unsupported query: {:?}",
                            "needs to be global_config"
                        ),
                        request: Default::default(),
                    })
                }
            },

            QueryRequest::Wasm(WasmQuery::Smart {
                msg,
                ..
            }) => {
                // if let Ok(query) = from_binary::<Cw20QueryMsg>(msg) {
                //     return self.cw20_querier.handle_query(contract_addr, query);
                // }

                err_unsupported_query(msg)
            },

            QueryRequest::Bank(query) => self.bank_querier.query(query),

            _ => err_unsupported_query(request),
        }
    }
}

pub(super) fn err_unsupported_query<T: std::fmt::Debug>(request: T) -> QuerierResult {
    SystemResult::Err(SystemError::InvalidRequest {
        error: format!("[mock] unsupported query: {:?}", request),
        request: Default::default(),
    })
}
