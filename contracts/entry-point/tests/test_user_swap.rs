use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin, ContractResult, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use skip::{
    entry_point::{Affiliate, ExecuteMsg},
    error::SkipError::{
        SwapOperationsCoinInDenomMismatch, SwapOperationsCoinOutDenomMismatch, SwapOperationsEmpty,
    },
    swap::{ExecuteMsg as SwapExecuteMsg, Swap, SwapExactCoinIn, SwapOperation},
};
use skip_swap_entry_point::{error::ContractError, state::SWAP_VENUE_MAP};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - User Swap With No Affiliates
    - User Swap With Single Affiliate
    - User Swap With Multiple Affiliates

Expect Error
    - User Swap First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom
    - User Swap Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom
    - Empty User Swap Operations
    - Unauthorized Caller

 */

// Define test parameters
struct Params {
    caller: String,
    user_swap: Swap,
    remaining_coin: Coin,
    min_coin: Coin,
    affiliates: Vec<Affiliate>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap_and_action
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                    }
                ],
            }
        ),
        remaining_coin: Coin::new(1_000_000, "untrn"),
        min_coin: Coin::new(1_000_000, "osmo"),
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "untrn".to_string(),
                                denom_out: "osmo".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "untrn")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap With No Affiliates")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                    }
                ],
            }
        ),
        remaining_coin: Coin::new(1_000_000, "untrn"),
        min_coin: Coin::new(1_000_000, "osmo"),
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "untrn".to_string(),
                                denom_out: "osmo".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "untrn")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap With Single Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                    }
                ],
            }
        ),
        remaining_coin: Coin::new(1_000_000, "untrn"),
        min_coin: Coin::new(1_000_000, "osmo"),
        affiliates: vec![
            Affiliate {
                address: "affiliate_1".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
            Affiliate {
                address: "affiliate_2".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "untrn".to_string(),
                                denom_out: "osmo".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "untrn")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate_1".to_string(),
                    amount: vec![Coin::new(100_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate_2".to_string(),
                    amount: vec![Coin::new(100_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap With Multiple Affiliates")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "uatom".to_string(),
                    }
                ],
            },
        ),
        remaining_coin: Coin::new(1_000_000, "uosmo"),
        min_coin: Coin::new(100_000, "uatom"),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsCoinInDenomMismatch)),
    };
    "User Swap First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "osmo".to_string(),
                    }
                ],
            },
        ),
        remaining_coin: Coin::new(1_000_000, "osmo"),
        min_coin: Coin::new(100_000, "uatom"),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsCoinOutDenomMismatch)),
    };
    "User Swap Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![],
            },
        ),
        remaining_coin: Coin::new(1_000_000, "untrn"),
        min_coin: Coin::new(1_000_000, "osmo"),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsEmpty)),
    };
    "Empty User Swap Operations - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        user_swap: Swap::SwapExactCoinIn (
            SwapExactCoinIn {
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![],
            },
        ),
        remaining_coin: Coin::new(1_000_000, "osmo"),
        min_coin: Coin::new(1_000_000, "osmo"),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
fn test_execute_user_swap(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock wasm handler to handle the swap adapter contract query
    // Will always return 200_000 osmo
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&Coin::new(200_000, "osmo")).unwrap(),
            )),
            _ => panic!("Unsupported query: {:?}", query),
        }
    };

    // Update querier with mock wasm handler
    deps.querier.update_wasm(wasm_handler);

    // Create mock env with parameters that make testing easier
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("entry_point");
    env.block.time = Timestamp::from_nanos(100);

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, &[]);

    // Store the ibc transfer adapter contract address
    let swap_venue_adapter = Addr::unchecked("swap_venue_adapter");
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name",
            &swap_venue_adapter,
        )
        .unwrap();

    // Call execute_swap_and_action with the given test case params
    let res = skip_swap_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UserSwap {
            swap: params.user_swap,
            remaining_coin: params.remaining_coin,
            min_coin: params.min_coin,
            affiliates: params.affiliates,
        },
    );

    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the number of messages in the response is correct
            assert_eq!(
                res.messages.len(),
                params.expected_messages.len(),
                "expected {:?} messages, but got {:?}",
                params.expected_messages.len(),
                res.messages.len()
            );

            // Assert the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages,);
        }
        Err(err) => {
            // Assert the test expected an error
            assert!(
                params.expected_error.is_some(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert_eq!(err, params.expected_error.unwrap());
        }
    }
}