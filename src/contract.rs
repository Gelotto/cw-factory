use crate::error::ContractError;
use crate::execute::create::{exec_create, exec_create_reply_handler};
use crate::execute::update::exec_update;
use crate::execute::{set_config::exec_set_config, Context};
use crate::msg::{ContractQueryMsg, ContractSetQueryMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::contract::has_relations::query_contract_has_relations;
use crate::query::contract::has_tags::query_contract_has_tags;
use crate::query::contract::relations::query_contract_relations;
use crate::query::contract::tags::query_contract_tags;
use crate::query::contracts::in_range::query_contracts_in_range;
use crate::query::contracts::related_to::query_contracts_related_to;
use crate::query::contracts::with_tag::query_contracts_with_tag;
use crate::query::{config::query_config, ReadonlyContext};
use crate::state;
use cosmwasm_std::{entry_point, to_json_binary as to_binary, Reply};
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
        QueryMsg::Config {} => to_binary(&query_config(ctx)?),
        QueryMsg::Contract(msg) => match msg {
            ContractQueryMsg::Tags(params) => to_binary(&query_contract_tags(ctx, params)?),
            ContractQueryMsg::HasTags(params) => to_binary(&query_contract_has_tags(ctx, params)?),
            ContractQueryMsg::Relations(params) => to_binary(&query_contract_relations(ctx, params)?),
            ContractQueryMsg::HasRelations(params) => to_binary(&query_contract_has_relations(ctx, params)?),
        },
        QueryMsg::Contracts(msg) => match msg {
            ContractSetQueryMsg::InRange(params) => to_binary(&query_contracts_in_range(ctx, params)?),
            ContractSetQueryMsg::WithTag(params) => to_binary(&query_contracts_with_tag(ctx, params)?),
            ContractSetQueryMsg::RelatedTo(params) => to_binary(&query_contracts_related_to(ctx, params)?),
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
