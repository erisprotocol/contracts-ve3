use cosmwasm_schema::write_api;
use ve3_shared::msgs_bribe_manager::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
  write_api! {
      instantiate: InstantiateMsg,
      execute: ExecuteMsg,
      query: QueryMsg,
  }
}
