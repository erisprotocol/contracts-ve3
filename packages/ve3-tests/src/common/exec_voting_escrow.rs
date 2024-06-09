use super::suite::TestingSuite;
use cosmwasm_std::{to_json_binary, Addr, Binary, Coin, ContractInfoResponse, StdResult, Uint128};
use cw20::{Cw20ReceiveMsg, Expiration, MinterResponse};
use cw721::{
  AllNftInfoResponse, ApprovalResponse, ApprovalsResponse, NftInfoResponse, NumTokensResponse,
  OperatorsResponse, OwnerOfResponse, TokensResponse,
};
use cw_asset::Asset;
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{extensions::asset_ext::AssetExt, helpers::time::Time, msgs_voting_escrow::*};

#[allow(dead_code)]
impl TestingSuite {
  fn contract(&self) -> Addr {
    self.addresses.ve3_voting_escrow.clone()
  }

  pub fn e_ve_create_lock_time(
    &mut self,
    time: u64,
    funds: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::CreateLock {
      time: Some(time),
    };

    match &funds.info {
      cw_asset::AssetInfoBase::Native(_) => {
        let coin: Coin = funds.to_coin().unwrap();
        let sender = self.address(sender);
        result(self.app.execute_contract(sender, self.contract(), &msg, &[coin]));
      },
      cw_asset::AssetInfoBase::Cw20(addr) => {
        let send_msg = cw20_base::msg::ExecuteMsg::Send {
          contract: self.contract().to_string(),
          amount: funds.amount,
          msg: to_json_binary(&msg).unwrap(),
        };

        let sender = self.address(sender);
        result(self.app.execute_contract(sender, addr.clone(), &send_msg, &[]));
      },
      _ => panic!("not supported"),
    }

    self
  }

  pub fn e_ve_create_lock_time_any(
    &mut self,
    time: Option<u64>,
    funds: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::CreateLock {
      time,
    };

    match &funds.info {
      cw_asset::AssetInfoBase::Native(_) => {
        let coin: Coin = funds.to_coin().unwrap();
        let sender = self.address(sender);
        result(self.app.execute_contract(sender, self.contract(), &msg, &[coin]));
      },
      cw_asset::AssetInfoBase::Cw20(addr) => {
        let send_msg = cw20_base::msg::ExecuteMsg::Send {
          contract: self.contract().to_string(),
          amount: funds.amount,
          msg: to_json_binary(&msg).unwrap(),
        };

        let sender = self.address(sender);
        result(self.app.execute_contract(sender, addr.clone(), &send_msg, &[]));
      },
      _ => panic!("not supported"),
    }

    self
  }

  pub fn e_ve_extend_lock_amount(
    &mut self,
    token_id: &str,
    sender: &str,
    funds: Asset,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ExtendLockAmount {
      token_id: token_id.to_string(),
    };

    match &funds.info {
      cw_asset::AssetInfoBase::Native(_) => {
        let coin: Coin = funds.to_coin().unwrap();
        let sender = self.address(sender);
        result(self.app.execute_contract(sender, self.contract(), &msg, &[coin]));
      },
      cw_asset::AssetInfoBase::Cw20(addr) => {
        let send_msg = cw20_base::msg::ExecuteMsg::Send {
          contract: self.contract().to_string(),
          amount: funds.amount,
          msg: to_json_binary(&msg).unwrap(),
        };

        let sender = self.address(sender);
        result(self.app.execute_contract(sender, addr.clone(), &send_msg, &[]));
      },
      _ => panic!("not supported"),
    }

    self
  }

  pub fn e_ve_merge_lock(
    &mut self,
    token_id: &str,
    token_id_add: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::MergeLock {
      token_id: token_id.to_string(),
      token_id_add: token_id_add.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_split_lock(
    &mut self,
    token_id: &str,
    amount: Uint128,
    recipient: Option<&str>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::SplitLock {
      token_id: token_id.to_string(),
      amount,
      recipient: recipient.map(|r| self.address(r).to_string()),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_extend_lock_time(
    &mut self,
    time: u64,
    token_id: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ExtendLockTime {
      time,
      token_id: token_id.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_withdraw(
    &mut self,
    token_id: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Withdraw {
      token_id: token_id.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_receive(
    &mut self,
    cw20_receive_msg: Cw20ReceiveMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Receive(cw20_receive_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_lock_permanent(
    &mut self,
    token_id: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::LockPermanent {
      token_id: token_id.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_unlock_permanent(
    &mut self,
    token_id: String,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UnlockPermanent {
      token_id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_update_blacklist(
    &mut self,
    append_addrs: Option<Vec<String>>,
    remove_addrs: Option<Vec<String>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateBlacklist {
      append_addrs,
      remove_addrs,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_update_config(
    &mut self,
    append_deposit_assets: Option<Vec<DepositAsset<String>>>,
    push_update_contracts: Option<Vec<String>>,
    decommissioned: Option<bool>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateConfig {
      append_deposit_assets,
      push_update_contracts,
      decommissioned,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_transfer_nft(
    &mut self,
    recipient: String,
    token_id: String,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::TransferNft {
      recipient,
      token_id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_send_nft(
    &mut self,
    contract: String,
    token_id: String,
    msg: Binary,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::SendNft {
      contract,
      token_id,
      msg,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_burn(
    &mut self,
    token_id: String,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Burn {
      token_id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_approve(
    &mut self,
    spender: &str,
    token_id: String,
    expires: Option<Expiration>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Approve {
      spender: self.address(spender).to_string(),
      token_id,
      expires,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_revoke(
    &mut self,
    spender: String,
    token_id: String,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Revoke {
      spender,
      token_id,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn e_ve_approve_all(
    &mut self,
    operator: String,
    expires: Option<Expiration>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ApproveAll {
      operator,
      expires,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub fn ve_revoke_all(
    &mut self,
    operator: String,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::RevokeAll {
      operator,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract(), &msg, &[]));
    self
  }

  pub(crate) fn q_ve_lock_vamp(
    &mut self,
    token_id: String,
    time: Option<Time>,
    result: impl Fn(StdResult<VotingPowerResponse>),
  ) -> &mut Self {
    let incentive_response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::LockVamp {
        token_id,
        time,
      },
    );

    result(incentive_response);

    self
  }

  pub(crate) fn q_ve_blacklisted_voters(
    &mut self,
    start_after: Option<String>,
    limit: Option<u32>,
    result: impl Fn(StdResult<Vec<Addr>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::BlacklistedVoters {
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_total_vamp(
    &mut self,
    time: Option<Time>,
    result: impl Fn(StdResult<VotingPowerResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::TotalVamp {
        time,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_lock_info(
    &mut self,
    token_id: &str,
    time: Option<Time>,
    result: impl Fn(StdResult<LockInfoResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::LockInfo {
        token_id: token_id.to_string(),
        time,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub(crate) fn q_ve_nft_info(
    &mut self,
    token_id: String,
    result: impl Fn(StdResult<NftInfoResponse<Extension>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::NftInfo {
        token_id,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_all_nft_info(
    &mut self,
    token_id: String,
    include_expired: Option<bool>,
    result: impl Fn(StdResult<AllNftInfoResponse<Extension>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::AllNftInfo {
        token_id,
        include_expired,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_owner_of(
    &mut self,
    token_id: String,
    include_expired: Option<bool>,
    result: impl Fn(StdResult<OwnerOfResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::OwnerOf {
        token_id,
        include_expired,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_approval(
    &mut self,
    token_id: String,
    spender: String,
    include_expired: Option<bool>,
    result: impl Fn(StdResult<ApprovalResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::Approval {
        token_id,
        spender,
        include_expired,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_approvals(
    &mut self,
    token_id: String,
    include_expired: Option<bool>,
    result: impl Fn(StdResult<ApprovalsResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::Approvals {
        token_id,
        include_expired,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_all_operators(
    &mut self,
    owner: String,
    include_expired: Option<bool>,
    start_after: Option<String>,
    limit: Option<u32>,
    result: impl Fn(StdResult<OperatorsResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::AllOperators {
        owner,
        include_expired,
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_num_tokens(
    &mut self,
    result: impl Fn(StdResult<NumTokensResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract(), &QueryMsg::NumTokens {});
    result(response);
    self
  }

  pub(crate) fn q_ve_contract_info(
    &mut self,
    result: impl Fn(StdResult<ContractInfoResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract(), &QueryMsg::ContractInfo {});
    result(response);
    self
  }

  pub(crate) fn q_ve_tokens(
    &mut self,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
    result: impl Fn(StdResult<TokensResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::Tokens {
        owner,
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_all_tokens(
    &mut self,
    start_after: Option<String>,
    limit: Option<u32>,
    result: impl Fn(StdResult<TokensResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract(),
      &QueryMsg::AllTokens {
        start_after,
        limit,
      },
    );
    result(response);
    self
  }

  pub(crate) fn q_ve_minter(&mut self, result: impl Fn(StdResult<MinterResponse>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract(), &QueryMsg::Minter {});
    result(response);
    self
  }
}
