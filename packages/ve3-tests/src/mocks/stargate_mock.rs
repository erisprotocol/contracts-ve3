// source from https://github.com/White-Whale-Defi-Platform/white-whale-core/blob/release/v2_contracts/packages/white-whale-testing/src/multi_test/stargate_mock.rs

use std::{fmt::Debug, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{
  coins, to_json_binary, Addr, Api, BankMsg, Binary, BlockInfo, CustomQuery, Empty, Querier,
  Storage, SubMsgResponse, Uint128,
};
use cw_multi_test::{
  AppResponse, BankSudo, CosmosRouter, Module, Stargate, StargateMsg, StargateQuery,
};
use ve3_shared::helpers::denom::{MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint};

pub struct StargateMockModule {}

impl Stargate for StargateMockModule {}

impl Module for StargateMockModule {
  type ExecT = StargateMsg;
  type QueryT = StargateQuery;
  type SudoT = Empty;

  fn execute<ExecC, QueryC>(
    &self,
    api: &dyn Api,
    storage: &mut dyn Storage,
    router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
    block: &BlockInfo,
    sender: Addr,
    msg: Self::ExecT,
  ) -> AnyResult<AppResponse>
  where
    ExecC:
      cosmwasm_std::CustomMsg + Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    QueryC: CustomQuery + DeserializeOwned + 'static,
  {
    let type_url = msg.type_url;
    match type_url.as_str() {
      "/osmosis.tokenfactory.v1beta1.MsgCreateDenom"
      | "/cosmwasm.tokenfactory.v1beta1.MsgCreateDenom"
      | "/injective.tokenfactory.v1beta1.MsgCreateDenom" => {
        let tf_msg: MsgCreateDenom = msg.value.try_into()?;

        // let new = format!("factory/{}/{}", tf_msg.sender, tf_msg.subdenom);
        // println!("CREATE MESSAGE DENOM: {0}", new);

        let astroport_response = astroport::token_factory::MsgCreateDenomResponse {
          new_token_denom: format!("factory/{}/{}", tf_msg.sender, tf_msg.subdenom),
        };

        let _old_response = to_json_binary(&MsgCreateDenomResponse {
          new_token_denom: format!("factory/{}/{}", tf_msg.sender, tf_msg.subdenom),
        })?;

        let submsg_response = SubMsgResponse {
          events: vec![],
          data: Some(astroport_response.into()),
        };
        Ok(submsg_response.into())
      },
      "/osmosis.tokenfactory.v1beta1.MsgMint"
      | "/cosmwasm.tokenfactory.v1beta1.MsgMint"
      | "/injective.tokenfactory.v1beta1.MsgMint" => {
        let tf_msg: MsgMint = msg.value.try_into()?;
        let mint_coins = tf_msg.amount.unwrap_or_default();
        let bank_sudo = BankSudo::Mint {
          to_address: tf_msg.mint_to_address,
          amount: coins(Uint128::from_str(&mint_coins.amount).unwrap().u128(), mint_coins.denom),
        };
        router.sudo(api, storage, block, bank_sudo.into())
      },
      "/osmosis.tokenfactory.v1beta1.MsgBurn"
      | "/cosmwasm.tokenfactory.v1beta1.MsgBurn"
      | "/injective.tokenfactory.v1beta1.MsgBurn" => {
        let tf_msg: MsgBurn = msg.value.try_into()?;
        let burn_coins = tf_msg.amount.unwrap_or_default();
        let burn_msg = BankMsg::Burn {
          amount: coins(Uint128::from_str(&burn_coins.amount).unwrap().u128(), burn_coins.denom),
        };
        router.execute(api, storage, block, Addr::unchecked(tf_msg.sender), burn_msg.into())
      },
      _ => Err(anyhow::anyhow!("Unexpected exec msg {type_url} from {sender:?}",)),
    }
  }

  fn query(
    &self,
    _api: &dyn Api,
    _storage: &dyn Storage,
    _querier: &dyn Querier,
    _block: &BlockInfo,
    _request: Self::QueryT,
  ) -> AnyResult<Binary> {
    todo!()
  }

  fn sudo<ExecC, QueryC>(
    &self,
    _api: &dyn Api,
    _storage: &mut dyn Storage,
    _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
    _block: &BlockInfo,
    _msg: Self::SudoT,
  ) -> AnyResult<AppResponse>
  where
    ExecC: cosmwasm_std::CustomMsg + DeserializeOwned + 'static,
    QueryC: CustomQuery + DeserializeOwned + 'static,
  {
    todo!()
  }

  // fn query(
  //   &self,
  //   _api: &dyn Api,
  //   _storage: &dyn Storage,
  //   _querier: &dyn Querier,
  //   _block: &BlockInfo,
  //   path: String,
  //   _data: Binary,
  // ) -> AnyResult<Binary> {
  //   match path.as_str() {
  //     "/injective.tokenfactory.v1beta1.QueryParamsResponse" => {
  //       Ok(to_json_binary(&QueryParamsResponse {
  //         params: Some(Params {
  //           denom_creation_fee: vec![coin(1_000_000, "uosmo")],
  //           denom_creation_gas_consume: 0,
  //         }),
  //       })?)
  //     },
  //     _ => Err(anyhow::anyhow!("Unexpected stargate query request {path}",)),
  //   }
  // }
}
