use crate::error::ContractError;
use crate::execute::create::{exec_create, handle_creation_reply};
use crate::execute::migrate::{
    exec_begin_migration, exec_cancel_migration, exec_migrate_one, exec_retry_migration, exec_step_migration,
    handle_migration_reply,
};
use crate::execute::set_preset::{exec_remove_preset, exec_set_preset};
use crate::execute::update::exec_update;
use crate::execute::{set_config::exec_set_config, Context};
use crate::msg::{
    ContractQueryMsg, ContractSetQueryMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, MigrationSessionMsg,
    MigrationsExecuteMsg, MigrationsQueryMsg, PresetsExecuteMsg, PresetsQueryMsg, QueryMsg,
};
use crate::query::contract::has_tags::query_contract_has_tags;
use crate::query::contract::is_related_to::query_contract_is_related_to;
use crate::query::contract::metadata::query_contract_metadata;
use crate::query::contract::relations::query_contract_relations;
use crate::query::contract::tags::query_contract_tags;
use crate::query::contracts::in_range::query_contracts_in_range;
use crate::query::contracts::related_to::query_contracts_related_to;
use crate::query::contracts::with_tag::query_contracts_with_tag;
use crate::query::migrations::query_migration_session;
use crate::query::presets::{query_paginated_presets, query_preset};
use crate::query::{config::query_config, ReadonlyContext};
use crate::state;
use crate::state::storage::MIGRATION_REPLY_ID_2_STATE;
use crate::util::ensure_is_manager;
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
        ExecuteMsg::Configure(config) => exec_set_config(ctx, config),
        ExecuteMsg::Create(msg) => exec_create(ctx, msg),
        ExecuteMsg::Update(msg) => exec_update(ctx, msg),
        ExecuteMsg::Migrations(msg) => {
            ensure_is_manager(ctx.deps.storage, &ctx.info.sender)?;
            match msg {
                MigrationsExecuteMsg::Migrate(params) => exec_migrate_one(ctx, params),
                MigrationsExecuteMsg::Session(msg) => match msg {
                    MigrationSessionMsg::Begin(params) => exec_begin_migration(ctx, params),
                    MigrationSessionMsg::Step { name } => exec_step_migration(ctx, name),
                    MigrationSessionMsg::Cancel { name } => exec_cancel_migration(ctx, name),
                    MigrationSessionMsg::Retry { name, params } => exec_retry_migration(ctx, name, params),
                },
            }
        },
        ExecuteMsg::Presets(msg) => {
            ensure_is_manager(ctx.deps.storage, &ctx.info.sender)?;
            match msg {
                PresetsExecuteMsg::Set(msg) => exec_set_preset(ctx, msg),
                PresetsExecuteMsg::Remove { name } => exec_remove_preset(ctx, name),
            }
        },
    }
}

#[entry_point]
pub fn reply(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    if MIGRATION_REPLY_ID_2_STATE.has(deps.storage, reply.id) {
        handle_migration_reply(deps, reply)
    } else {
        handle_creation_reply(deps, env, reply)
    }
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
        QueryMsg::Migrations(msg) => match msg {
            MigrationsQueryMsg::Session(name) => to_binary(&query_migration_session(ctx, name)?),
        },
        QueryMsg::Contract(msg) => match msg {
            ContractQueryMsg::Metadata { address } => to_binary(&query_contract_metadata(ctx, address)?),
            ContractQueryMsg::IsRelatedTo(params) => to_binary(&query_contract_is_related_to(ctx, params)?),
            ContractQueryMsg::HasTags(params) => to_binary(&query_contract_has_tags(ctx, params)?),
            ContractQueryMsg::Relations(params) => to_binary(&query_contract_relations(ctx, params)?),
            ContractQueryMsg::Tags(params) => to_binary(&query_contract_tags(ctx, params)?),
        },
        QueryMsg::Contracts(msg) => match msg {
            ContractSetQueryMsg::InRange(params) => to_binary(&query_contracts_in_range(ctx, params)?),
            ContractSetQueryMsg::WithTag(params) => to_binary(&query_contracts_with_tag(ctx, params)?),
            ContractSetQueryMsg::RelatedTo(params) => to_binary(&query_contracts_related_to(ctx, params)?),
        },
        QueryMsg::Presets(msg) => match msg {
            PresetsQueryMsg::Get { name } => to_binary(&query_preset(ctx, name)?),
            PresetsQueryMsg::Paginate { cursor } => to_binary(&query_paginated_presets(ctx, cursor)?),
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
