use crate::{
    error::{ContractError, ContractResult},
    msg::{
        HallswapExecuteMsg, HallswapInterface, HallswapQueryMsg, HallswapQuerySimulationResult,
        HallswapRouteInfo, HallswapSwapOperation,
    },
    state::{ENTRY_POINT_CONTRACT_ADDRESS, HALLSWAP_CONTRACT_ADDRESS},
};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo,
    Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    error::SkipError,
    swap::{
        Cw20HookMsg, ExecuteMsg, HallswapInstantiateMsg, MigrateMsg, QueryMsg, Route, SwapOperation,
    },
};

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HallswapInstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    // Validate hallswap contract address
    let checked_hallswap_contract_address =
        deps.api.addr_validate(&msg.hallswap_contract_address)?;

    // Store the entry point contract address
    HALLSWAP_CONTRACT_ADDRESS.save(deps.storage, &checked_hallswap_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////
/// RECEIVE ///
///////////////

// Receive is the main entry point for the contract to
// receive cw20 tokens and execute the swap
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    mut info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> ContractResult<Response> {
    let sent_asset = Asset::Cw20(Cw20Coin {
        address: info.sender.to_string(),
        amount: cw20_msg.amount,
    });
    sent_asset.validate(&deps, &env, &info)?;

    // Set the sender to the originating address that triggered the cw20 send call
    // This is later validated / enforced to be the entry point contract address
    info.sender = deps.api.addr_validate(&cw20_msg.sender)?;

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::Swap { routes } => execute_swap(deps, env, info, routes, sent_asset),
    }
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive_cw20(deps, env, info, cw20_msg),
        ExecuteMsg::Swap { routes } => {
            let coin = one_coin(&info)?;
            execute_swap(deps, env, info, routes, Asset::Native(coin))
        }
        _ => {
            unimplemented!()
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    routes: Vec<Route>,
    asset: Asset,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Create a response object to return
    let response: Response = Response::new().add_attribute("action", "execute_swap");

    // Get hallswap contract address
    let hallswap_contract_address = HALLSWAP_CONTRACT_ADDRESS.load(deps.storage)?;

    // Create hallswap swap message
    let hallswap_routes = get_hallswap_routes_from_skip_routes(deps.as_ref(), routes)?;
    let hallswap_execute_msg = HallswapExecuteMsg::ExecuteRoutesV2 {
        routes: hallswap_routes,
        minimum_receive: Uint128::zero(),
        to: Some(entry_point_contract_address),
    };
    let msg = match asset.clone() {
        Asset::Native(_) => WasmMsg::Execute {
            contract_addr: hallswap_contract_address.to_string(),
            msg: to_json_binary(&hallswap_execute_msg)?,
            funds: info.funds,
        },
        Asset::Cw20(cw20) => WasmMsg::Execute {
            contract_addr: cw20.address.to_string(),
            funds: vec![],
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: hallswap_contract_address.to_string(),
                amount: asset.amount(),
                msg: to_json_binary(&hallswap_execute_msg)?,
            })?,
        },
    };

    Ok(response
        .add_message(msg)
        .add_attribute("action", "dispatch_swaps_and_transfer_back"))
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in,
            swap_operations,
        } => to_json_binary(&query_simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?),
        _ => {
            unimplemented!()
        }
    }
    .map_err(From::from)
}

// Queries the hallswap contract to simulate a swap exact amount in
fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure asset_in's denom is the same as the first swap operation's denom in
    if asset_in.denom() != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // Get hallswap contract address
    let hallswap_contract_address = HALLSWAP_CONTRACT_ADDRESS.load(deps.storage)?;

    // Query hallswap contract to get simulation results
    let res: HallswapQuerySimulationResult = deps.querier.query_wasm_smart(
        hallswap_contract_address,
        &HallswapQueryMsg::Simulation {
            routes: get_hallswap_routes_from_skip_routes(
                deps,
                vec![Route {
                    offer_asset: asset_in,
                    operations: swap_operations,
                }],
            )?,
        },
    )?;
    let return_asset = res.return_asset;

    // Return the asset out
    match return_asset.info {
        astroport::asset::AssetInfo::Token { contract_addr } => Ok(Asset::Cw20(Cw20Coin {
            address: contract_addr.to_string(),
            amount: return_asset.amount,
        })),
        astroport::asset::AssetInfo::NativeToken { denom } => {
            Ok(Asset::Native(Coin::new(return_asset.amount.into(), denom)))
        }
    }
}

fn get_hallswap_routes_from_skip_routes(
    deps: Deps,
    routes: Vec<Route>,
) -> Result<Vec<HallswapRouteInfo>, SkipError> {
    routes
        .iter()
        .map(|route| {
            let hallswap_operations = route
                .operations
                .iter()
                .map(|op| {
                    Ok(HallswapSwapOperation {
                        contract_addr: deps.api.addr_validate(&op.pool)?,
                        offer_asset: Asset::new(deps.api, &op.denom_in, Uint128::zero())
                            .into_astroport_asset(deps.api)?
                            .info,
                        return_asset: Asset::new(deps.api, &op.denom_out, Uint128::zero())
                            .into_astroport_asset(deps.api)?
                            .info,
                        interface: op
                            .interface
                            .as_ref()
                            .map(|interface| HallswapInterface::Binary(interface.clone())),
                    })
                })
                .collect::<Result<Vec<HallswapSwapOperation>, SkipError>>()?;

            Ok(HallswapRouteInfo {
                route: hallswap_operations,
                offer_amount: route.offer_asset.amount(),
            })
        })
        .collect()
}
