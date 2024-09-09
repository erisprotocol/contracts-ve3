use crate::{
  common::{
    helpers::{u, uluna},
    suite::TestingSuite,
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal, StdError};
use cw_asset::Asset;
use eris::constants::{HOUR, WEEK};
use phoenix_treasury::error::ContractError;
use ve3_shared::{
  constants::{AT_DELEGATION_CONTROLLER, PDT_CONFIG_OWNER, PDT_CONTROLLER},
  error::SharedError,
  msgs_phoenix_treasury::*,
};

#[test]
fn test_permissions() {
  let mut suite = TestingSuite::def();
  suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_pdt_alliance_delegate(
      AllianceDelegateMsg {
        delegations: vec![],
      },
      "user1",
      |res| {
        res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
          AT_DELEGATION_CONTROLLER.to_string(),
          addr.user1.to_string(),
        )))
      },
    )
    .e_pdt_alliance_delegate(
      AllianceDelegateMsg {
        delegations: vec![],
      },
      "AT_DELEGATION_CONTROLLER",
      |res| res.assert_error(ContractError::EmptyDelegation {}),
    );
}

#[test]
fn test_payment() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![],
      },
      "user1",
      |res| {
        res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
          PDT_CONTROLLER.to_string(),
          addr.user1.to_string(),
        )))
      },
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_error(ContractError::ActionNotReservingAnyFunds);
      },
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![("user1".to_string(), addr.uluna(1000)).into()],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_error(ContractError::Std(StdError::generic_err("Invalid input")));
      },
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_error(ContractError::NotEnoughFunds(u(0), addr.uluna(1000)));
      },
    )
    .def_send("user1", addr.pdt.clone(), addr.uluna(100_000_000))
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_error(ContractError::MissingOracle(addr.uluna_info_checked()));
      },
    )
    .e_pdt_update_config(None, None, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        PDT_CONFIG_OWNER.to_string(),
        addr.user1.to_string(),
      )));
    })
    .e_pdt_update_config(None, None, PDT_CONFIG_OWNER, |res| {
      res.assert_valid();
    })
    .e_pdt_update_config(
      Some(vec![
        (
          addr.uluna_info(),
          Oracle::Pair {
            contract: addr.astroport_pair_mock.to_string(),
            simulation_amount: u(1_000000),
          },
        ),
        (addr.usdc_info(), Oracle::Usdc),
      ]),
      None,
      PDT_CONFIG_OWNER,
      |res| {
        res.assert_valid();
      },
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .def_send("user1", addr.pdt.clone(), addr.usdc(100_000_000))
    .e_pdt_setup(
      "test2",
      TreasuryActionSetup::Payment {
        payments: vec![
          (addr.user1.to_string(), addr.usdc(500)).into(),
          (addr.user2.to_string(), addr.uluna(1000)).into(),
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "2"));
      },
    )
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 1,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.uluna(1000).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
            },
            total_usd: u(300),
            total_usd_30d: u(300),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.uluna(1000)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 2,
            active_from: current_time,
            name: "test2".to_string(),
            reserved: vec![addr.usdc(500), addr.uluna(1000)].into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![
                (addr.user1.to_string(), addr.usdc(500)).into(),
                (addr.user2.to_string(), addr.uluna(1000)).into()
              ],
            },
            total_usd: u(800),
            total_usd_30d: u(1100),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![
                (addr.user1.to_string(), addr.usdc(500)).into(),
                (addr.user2.to_string(), addr.uluna(1000)).into()
              ]
            }
          }
        ]
      )
    })
    .q_pdt_actions_direction(None, None, Direction::Desc, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 2,
            active_from: current_time,
            name: "test2".to_string(),
            reserved: vec![addr.usdc(500), addr.uluna(1000)].into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![
                (addr.user1.to_string(), addr.usdc(500)).into(),
                (addr.user2.to_string(), addr.uluna(1000)).into()
              ],
            },
            total_usd: u(800),
            total_usd_30d: u(1100),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![
                (addr.user1.to_string(), addr.usdc(500)).into(),
                (addr.user2.to_string(), addr.uluna(1000)).into()
              ]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 1,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.uluna(1000).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
            },
            total_usd: u(300),
            total_usd_30d: u(300),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.uluna(1000)).into()]
            }
          }
        ]
      )
    })
    .q_pdt_user_actions("user2", None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: false,
          id: 2,
          active_from: current_time,
          name: "test2".to_string(),
          reserved: vec![addr.usdc(500), addr.uluna(1000)].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(500)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ],
          },
          total_usd: u(800),
          total_usd_30d: u(1100),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![
              (addr.user1.to_string(), addr.usdc(500)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ]
          }
        }]
      )
    })
    .q_pdt_user_actions("user1", None, Some(1), |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: false,
          id: 2,
          active_from: current_time,
          name: "test2".to_string(),
          reserved: vec![addr.usdc(500), addr.uluna(1000)].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(500)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ],
          },
          total_usd: u(800),
          total_usd_30d: u(1100),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![
              (addr.user1.to_string(), addr.usdc(500)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ]
          }
        }]
      )
    })
    .q_pdt_user_actions("user1", Some(2), Some(1), |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time,
          name: "test".to_string(),
          reserved: addr.uluna(1000).into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
          },
          total_usd: u(300),
          total_usd_30d: u(300),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![(addr.user1.to_string(), addr.uluna(1000)).into()]
          }
        }]
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 2,
          reserved: vec![addr.uluna(2000), addr.usdc(500)].into(),
          clawback: false,
        }
      )
    })
    .q_pdt_balances(None, |res| {
      assert_eq!(
        res.unwrap(),
        BalancesResponse {
          reserved: vec![addr.uluna(2000), addr.usdc(500)].into(),
          available: vec![addr.usdc(100000000 - 500), addr.uluna(100000000 - 2000)].into(),
        }
      )
    })
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.uluna(100000000)).into()],
      },
      PDT_CONTROLLER,
      |res| {
        // still checking for +2000 from reserved
        res.assert_error(ContractError::NotEnoughFunds(u(100000000), addr.uluna(100002000)));
      },
    )
    .e_pdt_claim(2, "anyone", |res| res.assert_error(ContractError::CannotClaimNoOpenPayment))
    .e_pdt_claim(2, "user2", |res| {
      res.assert_attribute(attr("action", "pdt/claim"));
      res.assert_transfer(addr.user2.to_string(), addr.uluna(1000));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 2,
          reserved: vec![addr.uluna(1000), addr.usdc(500)].into(),
          clawback: false,
        }
      )
    })
    .q_pdt_balances(None, |res| {
      assert_eq!(
        res.unwrap(),
        BalancesResponse {
          reserved: vec![addr.uluna(1000), addr.usdc(500)].into(),
          available: vec![addr.usdc(100000000 - 500), addr.uluna(100000000 - 2000)].into(),
        }
      )
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 1,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.uluna(1000).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
            },
            total_usd: u(300),
            total_usd_30d: u(300),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.uluna(1000)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 2,
            active_from: current_time,
            name: "test2".to_string(),
            reserved: vec![addr.usdc(500)].into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![
                (addr.user1.to_string(), addr.usdc(500)).into(),
                (addr.user2.to_string(), addr.uluna(1000)).into()
              ],
            },
            total_usd: u(800),
            total_usd_30d: u(1100),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(500)).into()]
            }
          }
        ]
      )
    })
    .e_pdt_claim(2, "user2", |res| {
      res.assert_error(ContractError::CannotClaimNoOpenPayment);
    })
    .e_pdt_claim(2, "user1", |res| {
      res.assert_attribute(attr("action", "pdt/claim"));
      res.assert_transfer(addr.user1.to_string(), addr.usdc(500));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 2,
          reserved: vec![addr.uluna(1000)].into(),
          clawback: false
        }
      )
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 1,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.uluna(1000).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.uluna(1000)).into()],
            },
            total_usd: u(300),
            total_usd_30d: u(300),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.uluna(1000)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: true,
            id: 2,
            active_from: current_time,
            name: "test2".to_string(),
            reserved: vec![].into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![
                (addr.user1.to_string(), addr.usdc(500)).into(),
                (addr.user2.to_string(), addr.uluna(1000)).into()
              ],
            },
            total_usd: u(800),
            total_usd_30d: u(1100),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![]
            }
          }
        ]
      )
    });
}

#[test]
fn test_payment_cancel_half_claimed() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![
          (addr.user1.to_string(), addr.usdc(500)).into(),
          (addr.user2.to_string(), addr.uluna(1000)).into(),
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_transfer(addr.user2.to_string(), addr.uluna(1000));
    })
    .e_pdt_cancel(1, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        PDT_CONTROLLER.to_string(),
        addr.user1.to_string(),
      )))
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.usdc(500)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_cancel(1, PDT_CONTROLLER, |res| {
      res.assert_attribute(attr("action", "pdt/cancel"));
      res.assert_attribute(attr("id", "1"));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::ActionCancelled(1));
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: true,
          done: false,
          id: 1,
          active_from: current_time,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(500)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ],
          },
          total_usd: u(800),
          total_usd_30d: u(800),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![(addr.user1.to_string(), addr.usdc(500)).into()]
          }
        }]
      )
    });
}

#[test]
fn test_payment_veto_half_claimed() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![
          (addr.user1.to_string(), addr.usdc(500)).into(),
          (addr.user2.to_string(), addr.uluna(1000)).into(),
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_transfer(addr.user2.to_string(), addr.uluna(1000));
    })
    .e_pdt_cancel(1, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        PDT_CONTROLLER.to_string(),
        addr.user1.to_string(),
      )))
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.usdc(500)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_veto(1, "veto-big", |res| {
      res.assert_error(ContractError::ActionValueNotEnough(u(1000), u(800)));
    })
    .e_pdt_veto(1, "veto-always", |res| {
      res.assert_attribute(attr("action", "pdt/veto"));
      res.assert_attribute(attr("id", "1"));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::ActionCancelled(1));
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: true,
          done: false,
          id: 1,
          active_from: current_time,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(500)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ],
          },
          total_usd: u(800),
          total_usd_30d: u(800),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![(addr.user1.to_string(), addr.usdc(500)).into()]
          }
        }]
      )
    });
}

#[test]
fn test_payment_delay() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![
          (addr.user1.to_string(), addr.usdc(500), current_time + 2 * WEEK).into(),
          (addr.user1.to_string(), addr.usdc(500), current_time + 4 * WEEK).into(),
          (addr.user1.to_string(), addr.usdc(500), current_time + 6 * WEEK).into(),
          (addr.user1.to_string(), addr.usdc(500), current_time + 8 * WEEK).into(),
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.usdc(2000)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .add_one_period()
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::CannotClaimNoOpenPayment);
    })
    .add_one_period()
    .e_pdt_claim(1, "user1", |result| {
      result.assert_transfer(addr.user1.to_string(), addr.usdc(500));
    })
    .add_one_period()
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::CannotClaimNoOpenPayment);
    })
    .add_one_period()
    .add_one_period()
    .add_one_period()
    .e_pdt_claim(1, "user1", |result| {
      result.assert_transfer(addr.user1.to_string(), addr.usdc(500));
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_transfer(addr.user1.to_string(), addr.usdc(500));
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::CannotClaimNoOpenPayment);
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.usdc(500)].into(),
          clawback: false
        }
      )
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.usdc(500)].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(500), current_time + 2 * WEEK).into(),
              (addr.user1.to_string(), addr.usdc(500), current_time + 4 * WEEK).into(),
              (addr.user1.to_string(), addr.usdc(500), current_time + 6 * WEEK).into(),
              (addr.user1.to_string(), addr.usdc(500), current_time + 8 * WEEK).into(),
            ],
          },
          total_usd: u(2000),
          total_usd_30d: u(2000),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![(addr.user1.to_string(), addr.usdc(500), current_time + 8 * WEEK).into()]
          }
        }
      )
    });
}

#[test]
fn test_veto_delay() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![
          (addr.user1.to_string(), addr.usdc(5000)).into(),
          (addr.user2.to_string(), addr.uluna(1000)).into(),
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.usdc(5000), addr.uluna(1000)].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(5000)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ],
          },
          total_usd: u(5300),
          total_usd_30d: u(5300),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![
              (addr.user1.to_string(), addr.usdc(5000)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ]
          }
        }]
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.usdc(5000), addr.uluna(1000)].into(),
          clawback: false
        }
      )
    })
    .add_one_period()
    .e_pdt_claim(1, "user2", |result| {
      result.assert_transfer(addr.user2.to_string(), addr.uluna(1000));
    })
    .e_pdt_veto(1, "veto-big", |res| {
      res.assert_attribute(attr("action", "pdt/veto"));
      res.assert_attribute(attr("id", "1"));
    })
    .e_pdt_veto(1, "veto-always", |res| {
      res.assert_error(ContractError::ActionCancelled(1));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::ActionCancelled(1));
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: true,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![
              (addr.user1.to_string(), addr.usdc(5000)).into(),
              (addr.user2.to_string(), addr.uluna(1000)).into()
            ],
          },
          total_usd: u(5300),
          total_usd_30d: u(5300),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![(addr.user1.to_string(), addr.usdc(5000)).into()]
          }
        }]
      )
    });
}

#[test]
fn test_milestones() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Milestone {
        recipient: addr.user1.to_string(),
        asset_info: addr.uluna_info_checked(),
        milestones: vec![],
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::ActionNotReservingAnyFunds),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Milestone {
        recipient: addr.user1.to_string(),
        asset_info: addr.uluna_info_checked(),
        milestones: vec![Milestone {
          text: "anything".to_string(),
          amount: u(0),
        }],
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::ActionNotReservingAnyFunds),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Milestone {
        recipient: addr.user1.to_string(),
        asset_info: addr.uluna_info_checked(),
        milestones: vec![
          Milestone {
            text: "testnet".to_string(),
            amount: u(1000),
          },
          Milestone {
            text: "mainnet".to_string(),
            amount: u(5000),
          },
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .add_one_period()
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotClaimNothingToClaim);
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna(6000)].into(),
          setup: TreasuryActionSetup::Milestone {
            recipient: addr.user1.to_string(),
            asset_info: addr.uluna_info_checked(),
            milestones: vec![
              Milestone {
                text: "testnet".to_string(),
                amount: u(1000),
              },
              Milestone {
                text: "mainnet".to_string(),
                amount: u(5000),
              },
            ]
          },
          total_usd: u(1800),
          total_usd_30d: u(1800),
          runtime: TreasuryActionRuntime::Milestone {
            milestones: vec![
              MilestoneRuntime {
                amount: u(1000),
                claimed: false,
                enabled: false,
              },
              MilestoneRuntime {
                amount: u(5000),
                claimed: false,
                enabled: false,
              },
            ]
          }
        }]
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna(6000)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotClaimNothingToClaim);
    })
    .e_pdt_update_milestone(1, 0, true, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        PDT_CONTROLLER.to_string(),
        addr.user1.to_string(),
      )))
    })
    .e_pdt_update_milestone(1, 10, true, PDT_CONTROLLER, |res| {
      res.assert_error(ContractError::MilestoneNotFound)
    })
    .e_pdt_update_milestone(1, 0, true, PDT_CONTROLLER, |res| {
      res.assert_attribute(attr("action", "pdt/update_milestone"));
      res.assert_attribute(attr("id", "1"));
    })
    .e_pdt_claim(1, "user2", |result| {
      // user 2 claims, but sent to the recipient of milestones (user 1)
      result.assert_transfer(addr.user1.to_string(), addr.uluna(1000));
    })
    .e_pdt_update_milestone(1, 0, false, PDT_CONTROLLER, |res| {
      res.assert_error(ContractError::MilestoneClaimed);
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna(5000)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_veto(1, "veto-big", |res| {
      res.assert_attribute(attr("action", "pdt/veto"));
      res.assert_attribute(attr("id", "1"));
    })
    .e_pdt_veto(1, "veto-always", |res| {
      res.assert_error(ContractError::ActionCancelled(1));
    })
    .e_pdt_update_milestone(1, 1, true, PDT_CONTROLLER, |res| {
      res.assert_error(ContractError::ActionCancelled(1));
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::ActionCancelled(1));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: true,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Milestone {
            recipient: addr.user1.to_string(),
            asset_info: addr.uluna_info_checked(),
            milestones: vec![
              Milestone {
                text: "testnet".to_string(),
                amount: u(1000),
              },
              Milestone {
                text: "mainnet".to_string(),
                amount: u(5000),
              },
            ]
          },
          total_usd: u(1800),
          total_usd_30d: u(1800),
          runtime: TreasuryActionRuntime::Milestone {
            milestones: vec![
              MilestoneRuntime {
                amount: u(1000),
                claimed: true,
                enabled: true,
              },
              MilestoneRuntime {
                amount: u(5000),
                claimed: false,
                enabled: false,
              },
            ]
          }
        }]
      )
    });
}

#[test]
fn test_milestones_done() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Milestone {
        recipient: addr.user1.to_string(),
        asset_info: addr.uluna_info_checked(),
        milestones: vec![
          Milestone {
            text: "testnet".to_string(),
            amount: u(1000),
          },
          Milestone {
            text: "mainnet".to_string(),
            amount: u(5000),
          },
        ],
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .add_one_period()
    .e_pdt_update_milestone(1, 0, true, PDT_CONTROLLER, |res| {
      res.assert_attribute(attr("action", "pdt/update_milestone"));
      res.assert_attribute(attr("id", "1"));
    })
    .e_pdt_claim(1, "user2", |result| {
      // user 2 claims, but sent to the recipient of milestones (user 1)
      result.assert_transfer(addr.user1.to_string(), addr.uluna(1000));
    })
    .e_pdt_update_milestone(1, 1, true, PDT_CONTROLLER, |res| {
      res.assert_attribute(attr("action", "pdt/update_milestone"));
      res.assert_attribute(attr("id", "1"));
    })
    .e_pdt_claim(1, "user2", |result| {
      // user 2 claims, but sent to the recipient of milestones (user 1)
      result.assert_transfer(addr.user1.to_string(), addr.uluna(5000));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    })
    .e_pdt_veto(1, "veto-big", |res| {
      res.assert_error(ContractError::ActionDone(1));
    })
    .e_pdt_veto(1, "veto-always", |res| {
      res.assert_error(ContractError::ActionDone(1));
    })
    .e_pdt_update_milestone(1, 1, true, PDT_CONTROLLER, |res| {
      res.assert_error(ContractError::ActionDone(1));
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_error(ContractError::ActionDone(1));
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: true,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Milestone {
            recipient: addr.user1.to_string(),
            asset_info: addr.uluna_info_checked(),
            milestones: vec![
              Milestone {
                text: "testnet".to_string(),
                amount: u(1000),
              },
              Milestone {
                text: "mainnet".to_string(),
                amount: u(5000),
              },
            ]
          },
          total_usd: u(1800),
          total_usd_30d: u(1800),
          runtime: TreasuryActionRuntime::Milestone {
            milestones: vec![
              MilestoneRuntime {
                amount: u(1000),
                claimed: true,
                enabled: true,
              },
              MilestoneRuntime {
                amount: u(5000),
                claimed: true,
                enabled: true,
              },
            ]
          }
        }]
      )
    });
}

#[test]
fn test_otc() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Otc {
        amount: addr.uluna(0),
        into: addr.usdc(10000),
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::ActionNotReservingAnyFunds),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Otc {
        amount: addr.uluna(5000),
        into: addr.uluna(10000),
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::SwapAssetsSame),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Otc {
        amount: addr.uluna(10000),
        into: addr.usdc(1000),
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::OtcDiscountTooHigh(Decimal::percent(50))),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Otc {
        amount: addr.uluna(10000),
        into: addr.usdc(1500),
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .add_one_period()
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotClaimNotAllowed);
    })
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_error(ContractError::CannotExecuteOnlyDca);
    })
    .e_pdt_execute_otc_no_coins(1, u(0), "user1", |result| {
      result.assert_error(ContractError::CannotExecuteMissingFunds);
    })
    .e_pdt_execute_otc(1, addr.uluna(2000), "user1", |result| {
      result.assert_error(ContractError::SharedError(SharedError::WrongDeposit(
        "expected 2000ibc/usdc coins".to_string(),
      )));
    })
    .e_pdt_execute_otc(1, addr.usdc(3000), "user1", |result| {
      result.assert_error(ContractError::OtcAmountBiggerThanAvailable(u(20000), u(10000)));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna(10000)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_execute_otc(1, addr.usdc(750), "user1", |result| {
      result.assert_attribute(attr("action", "pdt/execute_otc"));
      result.assert_attribute(attr("id", "1"));
      result.assert_attribute(attr("returned", "5000"));
      result.assert_transfer(addr.user1.to_string(), addr.uluna(5000));
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna(5000)].into(),
          clawback: false
        }
      )
    })
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna(5000)].into(),
          setup: TreasuryActionSetup::Otc {
            amount: addr.uluna(10000),
            into: addr.usdc(1500),
          },
          total_usd: u(3000),
          total_usd_30d: u(3000),
          runtime: TreasuryActionRuntime::Otc {}
        }]
      )
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna(5000)].into(),
          setup: TreasuryActionSetup::Otc {
            amount: addr.uluna(10000),
            into: addr.usdc(1500),
          },
          total_usd: u(3000),
          total_usd_30d: u(3000),
          runtime: TreasuryActionRuntime::Otc {}
        }
      )
    })
    .e_pdt_execute_otc(1, addr.usdc(751), "user2", |result| {
      result.assert_error(ContractError::OtcAmountBiggerThanAvailable(u(5006), u(5000)));
    })
    .e_pdt_execute_otc(1, addr.usdc(750), "user2", |result| {
      result.assert_attribute(attr("action", "pdt/execute_otc"));
      result.assert_attribute(attr("id", "1"));
      result.assert_attribute(attr("returned", "5000"));
      result.assert_transfer(addr.user2.to_string(), addr.uluna(5000));
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: true,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Otc {
            amount: addr.uluna(10000),
            into: addr.usdc(1500),
          },
          total_usd: u(3000),
          total_usd_30d: u(3000),
          runtime: TreasuryActionRuntime::Otc {}
        }
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    });
}

#[test]
fn test_vesting() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Vesting {
        recipient: addr.user1.to_string(),
        amount: uluna(0),
        start_s: current_time,
        end_s: current_time + 10 * WEEK,
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::ActionNotReservingAnyFunds),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Vesting {
        recipient: addr.user1.to_string(),
        amount: uluna((20 * WEEK).try_into().unwrap()),
        start_s: current_time,
        end_s: current_time + 10 * WEEK,
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .add_one_period()
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna((WEEK * 20) as u32)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user2", |result| {
      result.assert_transfer(addr.user1.to_string(), addr.uluna((WEEK * 2) as u32));
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna((WEEK * 18) as u32)].into(),
          setup: TreasuryActionSetup::Vesting {
            recipient: addr.user1.to_string(),
            amount: addr.uluna((WEEK * 20) as u32),
            start_s: current_time,
            end_s: current_time + WEEK * 10
          },
          total_usd: u(3628800),
          total_usd_30d: u(3628800),
          runtime: TreasuryActionRuntime::Vesting {
            last_claim_s: current_time + WEEK
          }
        }
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna((WEEK * 18) as u32)].into(),
          clawback: false
        }
      )
    })
    .add_one_period()
    .add_one_period()
    .e_pdt_claim(1, "user1", |result| {
      result.assert_transfer(addr.user1.to_string(), addr.uluna((WEEK * 4) as u32));
    })
    .add_periods(50)
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna((WEEK * 14) as u32)].into(),
          setup: TreasuryActionSetup::Vesting {
            recipient: addr.user1.to_string(),
            amount: addr.uluna((WEEK * 20) as u32),
            start_s: current_time,
            end_s: current_time + WEEK * 10
          },
          total_usd: u(3628800),
          total_usd_30d: u(3628800),
          runtime: TreasuryActionRuntime::Vesting {
            last_claim_s: current_time + WEEK * 3
          }
        }
      )
    })
    .e_pdt_claim(1, "user1", |result| {
      result.assert_transfer(addr.user1.to_string(), addr.uluna((WEEK * 14) as u32));
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: true,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Vesting {
            recipient: addr.user1.to_string(),
            amount: addr.uluna((WEEK * 20) as u32),
            start_s: current_time,
            end_s: current_time + WEEK * 10
          },
          total_usd: u(3628800),
          total_usd_30d: u(3628800),
          runtime: TreasuryActionRuntime::Vesting {
            last_claim_s: current_time + WEEK * 53
          }
        }
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    });
}

#[test]
fn test_dca() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();
  let week = WEEK as u32;

  suite
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Dca {
        amount: addr.uluna(0),
        into: addr.usdc_info_checked(),
        max_per_swap: None,
        start_s: current_time + WEEK * 2,
        end_s: current_time + WEEK * 6,
        cooldown_s: HOUR,
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::ActionNotReservingAnyFunds),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Dca {
        amount: addr.uluna(week * 12),
        into: addr.usdc_info_checked(),
        max_per_swap: Some(u(week * 6)),
        start_s: current_time + WEEK * 2,
        end_s: current_time + WEEK * 6,
        cooldown_s: HOUR,
      },
      PDT_CONTROLLER,
      |res| {
        res.assert_attribute(attr("action", "pdt/setup"));
        res.assert_attribute(attr("id", "1"));
      },
    )
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_error(ContractError::CannotExecuteNotActive);
    })
    .add_one_period()
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![addr.uluna(week * 12)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_claim(1, "user2", |result| {
      result.assert_error(ContractError::CannotClaimNotAllowed);
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna(week * 12)].into(),
          setup: TreasuryActionSetup::Dca {
            amount: addr.uluna(week * 12),
            into: addr.usdc_info_checked(),
            max_per_swap: Some(u(week * 6)),
            start_s: current_time + WEEK * 2,
            end_s: current_time + WEEK * 6,
            cooldown_s: HOUR
          },
          total_usd: u(2177280),
          total_usd_30d: u(2177280),
          runtime: TreasuryActionRuntime::Dca {
            last_execution_s: current_time + WEEK * 2
          }
        }
      )
    })
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_error(ContractError::CannotExecuteDcaNotActive);
    })
    .add_one_period()
    .add_one_period()
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_attribute(attr("action", "pdt/execute_dca"));
      result.assert_attribute(attr("offer", addr.uluna(week * 3).to_string()));
      // week * 3 * 0.3
      // 604800 * 3 * 0.3
      result.assert_transfer(addr.pdt.to_string(), addr.usdc(544320));
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna(week * 9)].into(),
          setup: TreasuryActionSetup::Dca {
            amount: addr.uluna(week * 12),
            into: addr.usdc_info_checked(),
            max_per_swap: Some(u(week * 6)),
            start_s: current_time + WEEK * 2,
            end_s: current_time + WEEK * 6,
            cooldown_s: HOUR
          },
          total_usd: u(2177280),
          total_usd_30d: u(2177280),
          runtime: TreasuryActionRuntime::Dca {
            last_execution_s: current_time + WEEK * 3
          }
        }
      )
    })
    .add_one_period()
    .add_one_period()
    .add_one_period()
    .add_one_period()
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_attribute(attr("action", "pdt/execute_dca"));
      result.assert_attribute(attr("offer", addr.uluna(week * 6).to_string()));
      // week * 6 * 0.3
      // 604800 * 6 * 0.3
      result.assert_transfer(addr.pdt.to_string(), addr.usdc(544320 * 2));
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![addr.uluna(week * 3)].into(),
          setup: TreasuryActionSetup::Dca {
            amount: addr.uluna(week * 12),
            into: addr.usdc_info_checked(),
            max_per_swap: Some(u(week * 6)),
            start_s: current_time + WEEK * 2,
            end_s: current_time + WEEK * 6,
            cooldown_s: HOUR
          },
          total_usd: u(2177280),
          total_usd_30d: u(2177280),
          runtime: TreasuryActionRuntime::Dca {
            last_execution_s: current_time + WEEK * 7
          }
        }
      )
    })
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_error(ContractError::DcaWaitForCooldown(current_time + WEEK * 7 + HOUR));
    })
    .add_one_period()
    .e_pdt_execute_dca(1, None, "user1", |result| {
      result.assert_attribute(attr("action", "pdt/execute_dca"));
      result.assert_attribute(attr("offer", addr.uluna(week * 3).to_string()));
      result.assert_transfer(addr.pdt.to_string(), addr.usdc(544320));
    })
    .q_pdt_action(1, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: true,
          id: 1,
          active_from: current_time + WEEK,
          name: "test".to_string(),
          reserved: vec![].into(),
          setup: TreasuryActionSetup::Dca {
            amount: addr.uluna(week * 12),
            into: addr.usdc_info_checked(),
            max_per_swap: Some(u(week * 6)),
            start_s: current_time + WEEK * 2,
            end_s: current_time + WEEK * 6,
            cooldown_s: HOUR
          },
          total_usd: u(2177280),
          total_usd_30d: u(2177280),
          runtime: TreasuryActionRuntime::Dca {
            last_execution_s: current_time + WEEK * 8
          }
        }
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 1,
          reserved: vec![].into(),
          clawback: false
        }
      )
    });
}

#[test]
fn test_not_alliance() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let alliance_token_denom = format!("factory/{0}/vt", addr.pdt);
  let veto_owner = suite.address("PDT_VETO_CONFIG_OWNER");

  suite
    .q_pdt_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          reward_denom: "uluna".to_string(),
          alliance_token_denom: alliance_token_denom.clone(),
          vetos: vec![],
          veto_owner: veto_owner.clone()
        }
      )
    })
    .def_pdt()
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![
          (addr.user1.to_string(), Asset::native(alliance_token_denom, 1000u128)).into()
        ],
      },
      PDT_CONTROLLER,
      |res| res.assert_error(ContractError::CannotUseVt),
    );
}

#[test]
fn test_monthly_spend() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let current_time = suite.app.block_info().time.seconds();

  suite
    .def_pdt()
    .q_pdt_balances(None, |res| {
      assert_eq!(
        res.unwrap(),
        BalancesResponse {
          reserved: vec![].into(),
          available: vec![addr.usdc(100000000), addr.uluna(100000000),].into(),
        }
      )
    })
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 5,
          reserved: vec![addr.usdc(999 * 5)].into(),
          clawback: false
        }
      )
    })
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .q_pdt_actions(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 1,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.usdc(999).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
            },
            total_usd: u(999),
            total_usd_30d: u(999),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(999)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 2,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.usdc(999).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
            },
            total_usd: u(999),
            total_usd_30d: u(999 * 2),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(999)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 3,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.usdc(999).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
            },
            total_usd: u(999),
            total_usd_30d: u(999 * 3),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(999)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 4,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.usdc(999).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
            },
            total_usd: u(999),
            total_usd_30d: u(999 * 4),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(999)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 5,
            active_from: current_time,
            name: "test".to_string(),
            reserved: addr.usdc(999).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
            },
            total_usd: u(999),
            total_usd_30d: u(999 * 5),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(999)).into()]
            }
          },
          TreasuryAction {
            cancelled: false,
            done: false,
            id: 6,
            // delayed
            active_from: current_time + WEEK,
            name: "test".to_string(),
            reserved: addr.usdc(999).into(),
            setup: TreasuryActionSetup::Payment {
              payments: vec![(addr.user1.to_string(), addr.usdc(999)).into()],
            },
            total_usd: u(999),
            total_usd_30d: u(999 * 6),
            runtime: TreasuryActionRuntime::Payment {
              open: vec![(addr.user1.to_string(), addr.usdc(999)).into()]
            }
          },
        ]
      )
    })
    .add_periods(5)
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![(addr.user1.to_string(), addr.usdc(500)).into()],
      },
      PDT_CONTROLLER,
      |res| res.assert_valid(),
    )
    .q_pdt_action(7, |res| {
      assert_eq!(
        res.unwrap(),
        TreasuryAction {
          cancelled: false,
          done: false,
          id: 7,
          // delayed
          active_from: current_time + WEEK * 5,
          name: "test".to_string(),
          reserved: addr.usdc(500).into(),
          setup: TreasuryActionSetup::Payment {
            payments: vec![(addr.user1.to_string(), addr.usdc(500)).into()],
          },
          total_usd: u(500),
          total_usd_30d: u(500),
          runtime: TreasuryActionRuntime::Payment {
            open: vec![(addr.user1.to_string(), addr.usdc(500)).into()]
          }
        },
      )
    })
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 7,
          reserved: vec![addr.usdc(999 * 6 + 500)].into(),
          clawback: false
        }
      )
    })
    .q_pdt_balances(None, |res| {
      assert_eq!(
        res.unwrap(),
        BalancesResponse {
          reserved: vec![addr.usdc(999 * 6 + 500)].into(),
          available: vec![addr.usdc(100000000 - (999 * 6 + 500)), addr.uluna(100000000),].into(),
        }
      )
    });
}

#[test]
fn test_veto_update() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let alliance_token_denom = format!("factory/{0}/vt", addr.pdt);
  let veto_owner = suite.address("PDT_VETO_CONFIG_OWNER");
  let veto_always = suite.address("veto-always");
  let veto_big = suite.address("veto-big");

  suite
    .q_pdt_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          reward_denom: "uluna".to_string(),
          alliance_token_denom: alliance_token_denom.clone(),
          vetos: vec![],
          veto_owner: veto_owner.clone()
        }
      )
    })
    .def_pdt()
    .q_pdt_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          reward_denom: "uluna".to_string(),
          alliance_token_denom: alliance_token_denom.clone(),
          vetos: vec![
            VetoRight {
              vetoer: veto_always.clone(),
              spend_above_usd: u(0),
              spend_above_usd_30d: u(0),
              delay_s: 0,
            },
            VetoRight {
              vetoer: veto_big.clone(),
              spend_above_usd: u(1000),
              spend_above_usd_30d: u(5000),
              delay_s: WEEK,
            },
          ],
          veto_owner: veto_owner.clone()
        }
      )
    })
    .e_pdt_update_veto_config(vec![], "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::Unauthorized {}))
    })
    .e_pdt_update_veto_config(vec![], "PDT_VETO_CONFIG_OWNER", |res| res.assert_valid())
    .q_pdt_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          reward_denom: "uluna".to_string(),
          alliance_token_denom: alliance_token_denom.clone(),
          vetos: vec![],
          veto_owner: veto_owner.clone()
        }
      )
    });
}

#[test]
fn test_clawback() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .def_pdt()
    .e_pdt_clawback("user1", vec![], "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::Unauthorized {}))
    })
    .e_pdt_clawback(
      "user1",
      vec![addr.uluna_info(), addr.usdc_info()],
      "PDT_VETO_CONFIG_OWNER",
      |res| {
        res.assert_transfer(addr.user1.to_string(), addr.uluna(100000000));
        res.assert_transfer(addr.user1.to_string(), addr.usdc(100000000));
      },
    )
    .q_pdt_state(|res| {
      assert_eq!(
        res.unwrap(),
        State {
          max_id: 0,
          reserved: vec![].into(),
          clawback: true
        }
      )
    })
    .e_pdt_claim(1, "user2", |res| res.assert_error(ContractError::ClawbackTriggered))
    .e_pdt_setup(
      "test",
      TreasuryActionSetup::Payment {
        payments: vec![],
      },
      "user1",
      |res| res.assert_error(ContractError::ClawbackTriggered),
    );
}

#[test]
fn test_oracle_prices() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .def_pdt()
    .q_pdt_oracle_prices(None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          (addr.usdc_info_checked(), Decimal::one()),
          (addr.uluna_info_checked(), Decimal::percent(30))
        ]
      )
    })
    .q_pdt_oracle_prices(Some(vec![addr.uluna_info()]), |res| {
      assert_eq!(res.unwrap(), vec![(addr.uluna_info_checked(), Decimal::percent(30))])
    });
}

impl TestingSuite {
  pub fn def_pdt(&mut self) -> &mut TestingSuite {
    let addr = self.addresses.clone();
    let veto_always = self.address("veto-always");
    let veto_big = self.address("veto-big");
    self
      .def_send("user1", addr.pdt.clone(), addr.uluna(100_000_000))
      .def_send("user1", addr.pdt.clone(), addr.usdc(100_000_000))
      .e_pdt_update_config(
        Some(vec![
          (
            addr.uluna_info(),
            Oracle::Pair {
              contract: addr.astroport_pair_mock.to_string(),
              simulation_amount: u(1_000000),
            },
          ),
          (addr.usdc_info(), Oracle::Usdc),
        ]),
        None,
        PDT_CONFIG_OWNER,
        |res| {
          res.assert_valid();
        },
      )
      .e_pdt_update_veto_config(
        vec![
          VetoRight {
            vetoer: veto_always.to_string(),
            spend_above_usd: u(0),
            spend_above_usd_30d: u(0),
            delay_s: 0,
          },
          VetoRight {
            vetoer: veto_big.to_string(),
            spend_above_usd: u(1000),
            spend_above_usd_30d: u(5000),
            delay_s: WEEK,
          },
        ],
        "PDT_VETO_CONFIG_OWNER",
        |res| res.assert_valid(),
      );

    self
  }
}
