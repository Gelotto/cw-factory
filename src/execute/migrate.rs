use std::marker::PhantomData;

use crate::{
    error::ContractError,
    math::{add_u32, add_u64, sub_u32},
    msg::{MigrationParams, SingletonMigrationParams},
    state::{
        models::{Migration, MigrationError, MigrationErrorStrategy, MigrationStatus},
        storage::{
            CONTRACT_ADDR_2_ID, CONTRACT_ID_2_ADDR, ID_2_CODE_ID, MIGRATIONS, MIGRATION_ERRORS,
            MIGRATION_REPLY_ID_2_STATE, REPLY_ID_COUNTER,
        },
    },
};
use cosmwasm_std::{
    attr, to_json_binary, DepsMut, Empty, Event, Order, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult,
    WasmMsg,
};
use cw_storage_plus::Bound;

use super::Context;

const DEFAULT_LIMIT: u16 = 50;
const MAX_LIMIT: u16 = 100;

pub fn exec_migrate_one(
    ctx: Context,
    params: SingletonMigrationParams,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;
    let mut resp = Response::new().add_attributes(vec![attr("action", "migrate")]);
    let addr = deps.api.addr_validate(params.contract.as_str())?;
    let id = CONTRACT_ADDR_2_ID.load(deps.storage, &addr)?;

    // Ensure we're migrating from the required code ID
    if let Some(from_code_id) = params.from_code_id {
        let code_id = ID_2_CODE_ID.load(deps.storage, id)?;
        if u64::from_le_bytes(code_id.as_slice().try_into().unwrap()) != from_code_id.u64() {
            return Ok(resp);
        }
    }

    resp = resp.add_submessage(SubMsg::new(WasmMsg::Migrate {
        contract_addr: addr.to_string(),
        new_code_id: params.to_code_id.u64(),
        msg: params
            .migrate_msg
            .to_owned()
            .unwrap_or_else(|| to_json_binary(&Empty {}).unwrap()),
    }));

    Ok(resp)
}

pub fn exec_begin_migration(
    ctx: Context,
    params: MigrationParams,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;
    let mut params = params;

    // TODO: Validate and normalize params
    params.batch_size = Some(params.batch_size.unwrap_or(DEFAULT_LIMIT).clamp(0, MAX_LIMIT));

    MIGRATIONS.save(
        deps.storage,
        &params.name,
        &Migration {
            params: params.to_owned(),
            status: MigrationStatus::Running,
            cursor: None,
            retry_cursor: None,
            n_error: 0,
            n_success: 0,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "begin_migration"),
        attr("session_name", params.name.to_owned()),
    ]))
}

pub fn exec_step_migration(
    ctx: Context,
    session_name: String,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;

    let mut migration = MIGRATIONS.load(deps.storage, &session_name)?;

    if migration.status == MigrationStatus::Complete {
        return Err(ContractError::MigrationComplete { name: session_name });
    }

    let params = &migration.params;
    let batch_size = params.batch_size.unwrap();

    // Exclusive range bound to resume iteration from
    let min_bound = if let Some(cursor) = migration.cursor {
        Some(Bound::Exclusive((cursor, PhantomData)))
    } else {
        None
    };

    // Load all contrct ID/Addrs to process in this batch
    let entries: Vec<StdResult<_>> = CONTRACT_ID_2_ADDR
        .range(deps.storage, min_bound, None, Order::Ascending)
        .take(batch_size as usize)
        .collect();

    let mut migrate_submsgs: Vec<SubMsg> = Vec::with_capacity(entries.len());
    let mut next_cursor_id: Option<u32> = None;

    for result in entries {
        let (id, addr) = result?;
        next_cursor_id = Some(id);

        // Ensure we're migrating from the required code ID
        if let Some(from_code_id) = params.from_code_id {
            let code_id = ID_2_CODE_ID.load(deps.storage, id)?;
            if u64::from_le_bytes(code_id.as_slice().try_into().unwrap()) != from_code_id.u64() {
                continue;
            }
        }

        let reply_id = REPLY_ID_COUNTER
            .update(deps.storage, |n| -> Result<_, ContractError> { add_u64(n, 1u64) })?
            .u64()
            - 1;

        MIGRATION_REPLY_ID_2_STATE.save(deps.storage, reply_id, &(params.name.to_owned(), id))?;

        migrate_submsgs.push(SubMsg::reply_always(
            WasmMsg::Migrate {
                contract_addr: addr.to_string(),
                new_code_id: params.to_code_id.u64(),
                msg: params
                    .migrate_msg
                    .to_owned()
                    .unwrap_or_else(|| to_json_binary(&Empty {}).unwrap()),
            },
            reply_id,
        ))
    }

    if migrate_submsgs.len() < batch_size as usize {
        migration.status = MigrationStatus::Complete;
        migration.cursor = None;
    } else {
        migration.cursor = next_cursor_id;
    }

    MIGRATIONS.save(deps.storage, &params.name, &migration)?;

    Ok(Response::new().add_submessages(migrate_submsgs).add_attributes(vec![
        attr("action", "step_migration"),
        attr("session_name", session_name.to_owned()),
    ]))
}

pub fn exec_retry_migration(
    ctx: Context,
    session_name: String,
    override_migration_params: Option<MigrationParams>,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;

    let mut migration = MIGRATIONS.load(deps.storage, &session_name)?;
    let mut migrate_submsgs: Vec<SubMsg> = Vec::with_capacity(migration.params.batch_size.unwrap() as usize);
    let mut next_cursor_id: Option<u32> = None;

    // If we're using an updated params object when retrying the migration, then
    // reset the params field of the existing migration object.
    if let Some(mut override_params) = override_migration_params {
        // Don't let user set a custom name here, which could be out of sync
        // with the existing migration, which would mess things up.
        // TODO: Validate and normalize params
        override_params.name = migration.params.name;
        override_params.batch_size = Some(override_params.batch_size.unwrap_or(DEFAULT_LIMIT).clamp(0, MAX_LIMIT));
        migration.params = override_params;
    }

    let params = &migration.params;

    // Exclusive range bound to resume iteration from
    let min_bound = if let Some(cursor) = migration.retry_cursor {
        Some(Bound::Exclusive((cursor, PhantomData)))
    } else {
        None
    };

    let entries = MIGRATION_ERRORS
        .prefix(&session_name)
        .range(deps.storage, min_bound, None, Order::Ascending)
        .take(migration.params.batch_size.unwrap() as usize)
        .collect::<Vec<StdResult<_>>>();

    // Rerun migration on the next batch of contracts. Since we've already
    // checked that these contracts are coming from the expected code ID (if
    // specified in params), we don't need to check if the code ID ==
    // from_code_id or perform similar checks also covered in exec_migrate.
    for result in entries {
        let (id, error) = result?;
        next_cursor_id = Some(id);

        let reply_id = REPLY_ID_COUNTER
            .update(deps.storage, |n| -> Result<_, ContractError> { add_u64(n, 1u64) })?
            .u64()
            - 1;

        MIGRATION_REPLY_ID_2_STATE.save(deps.storage, reply_id, &(params.name.to_owned(), id))?;

        // Remove the error entry, which will be re-added in the reply
        // handler if it errors out again.
        MIGRATION_ERRORS.remove(deps.storage, (&params.name, id));

        migrate_submsgs.push(SubMsg::reply_always(
            WasmMsg::Migrate {
                contract_addr: error.contract.to_string(),
                new_code_id: params.to_code_id.u64(),
                msg: params
                    .migrate_msg
                    .to_owned()
                    .unwrap_or_else(|| to_json_binary(&Empty {}).unwrap()),
            },
            reply_id,
        ))
    }

    if migrate_submsgs.len() < params.batch_size.unwrap() as usize {
        migration.status = MigrationStatus::Complete;
        migration.cursor = None;
    } else {
        migration.cursor = next_cursor_id;
    }

    MIGRATIONS.save(deps.storage, &params.name, &migration)?;

    Ok(Response::new().add_submessages(migrate_submsgs).add_attributes(vec![
        attr("action", "retry_migration"),
        attr("session_name", session_name.to_owned()),
    ]))
}

pub fn exec_cancel_migration(
    ctx: Context,
    session_name: String,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;

    MIGRATIONS.remove(deps.storage, &session_name);

    for result in MIGRATION_ERRORS
        .prefix(&session_name)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<Vec<StdResult<_>>>()
    {
        let (id, error) = result?;
        MIGRATION_REPLY_ID_2_STATE.remove(deps.storage, error.reply_id.u64());
        MIGRATION_ERRORS.remove(deps.storage, (&session_name, id));
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "cancel_migration"),
        attr("session_name", session_name.to_owned()),
    ]))
}

pub fn handle_migration_reply(
    deps: DepsMut,
    reply: Reply,
) -> Result<Response, ContractError> {
    let (session_name, contract_id) = MIGRATION_REPLY_ID_2_STATE.load(deps.storage, reply.id)?;
    let contract_addr = CONTRACT_ID_2_ADDR.load(deps.storage, contract_id)?;

    let mut migration = MIGRATIONS.load(deps.storage, &session_name)?;
    let mut resp = Response::new();

    match reply.result {
        SubMsgResult::Ok(_) => {
            migration.n_success = add_u32(migration.n_success, 1)?;

            // If the migrated contract was previously tracked as "failed",
            // untrack it as such.
            if MIGRATION_ERRORS.has(deps.storage, (&session_name, contract_id)) {
                MIGRATION_ERRORS.remove(deps.storage, (&session_name, contract_id));
                migration.n_error = sub_u32(migration.n_error, 1)?;
            }

            resp = resp.add_event(Event::new("migration-success").add_attributes(vec![
                attr("migrated_contract_addr", contract_addr.to_string()),
                attr("session_name", session_name.to_owned()),
            ]));
        },
        SubMsgResult::Err(e) => {
            // Invalidate the entire tx on error if policy is "Abort"
            if let MigrationErrorStrategy::Abort = migration.params.error_strategy {
                return Err(ContractError::Std(StdError::generic_err(e.to_string())));
            }

            // Otherwise, track the malfunctioning contract address to be able
            // to retry later via retry_migration
            migration.n_error = add_u32(migration.n_error, 1)?;

            MIGRATION_ERRORS.save(
                deps.storage,
                (&session_name, contract_id),
                &MigrationError {
                    reply_id: reply.id.into(),
                    contract: contract_addr.to_owned(),
                    error: e.to_string(),
                },
            )?;

            resp = resp.add_event(Event::new("migration-error").add_attributes(vec![
                attr("migrated_contract_addr", contract_addr.to_string()),
                attr("session_name", session_name.to_owned()),
            ]));
        },
    };

    MIGRATIONS.save(deps.storage, &session_name, &migration)?;

    Ok(resp)
}
