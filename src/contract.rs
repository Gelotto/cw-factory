use crate::error::ContractError;
use crate::execute::create::{exec_create, exec_create_reply_handler};
use crate::execute::update::exec_update;
use crate::execute::{set_config::exec_set_config, Context};
use crate::msg::{ContractsQueryMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::contracts::query_contracts_by_index;
use crate::query::{config::query_config, ReadonlyContext};
use crate::state;
use cosmwasm_std::{entry_point, to_json_binary, Reply};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:cw-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(state::init(Context { deps, env, info }, msg)?)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = Context { deps, env, info };
    match msg {
        ExecuteMsg::SetConfig(config) => exec_set_config(ctx, config),
        ExecuteMsg::Create(msg) => exec_create(ctx, msg),
        ExecuteMsg::Update(msg) => exec_update(ctx, msg),
    }
}

#[entry_point]
pub fn reply(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let resp = exec_create_reply_handler(deps, env, reply)?;
    Ok(resp)
}

#[entry_point]
pub fn query(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    let ctx = ReadonlyContext { deps, env };
    let result = match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(ctx)?),
        QueryMsg::Contracts(msg) => match msg {
            ContractsQueryMsg::ByIndex(params) => to_json_binary(&query_contracts_by_index(ctx, params)?),
        },
    }?;
    Ok(result)
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
