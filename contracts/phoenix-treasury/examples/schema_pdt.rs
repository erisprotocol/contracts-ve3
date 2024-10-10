use cosmwasm_schema::write_api;
use ve3_shared::msgs_phoenix_treasury::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
  write_api! {
      instantiate: InstantiateMsg,
      execute: ExecuteMsg,
      query: QueryMsg,
      migrate: MigrateMsg
  }
}
