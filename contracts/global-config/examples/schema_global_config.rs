use cosmwasm_schema::write_api;
use ve3_shared::msgs_global_config::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
  write_api! {
      instantiate: InstantiateMsg,
      query: QueryMsg,
      execute: ExecuteMsg,
      migrate: MigrateMsg
  }
}