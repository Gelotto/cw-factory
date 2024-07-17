use std::marker::PhantomData;

use crate::{
    error::ContractError,
    math::{add_u32, add_u64},
    msg::MigrationParams,
    state::{
        models::{Migration, MigrationError, MigrationStatus},
        storage::{
            CONTRACT_ID_2_ADDR, ID_2_CODE_ID, MIGRATIONS, MIGRATION_ERRORS, MIGRATION_PARAMS,
            MIGRATION_REPLY_ID_2_STATE, REPLY_ID_COUNTER,
        },
    },
    util::ensure_is_manager,
};
use cosmwasm_std::{attr, DepsMut, Event, Order, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult, WasmMsg};
use cw_storage_plus::Bound;

use super::Context;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;

/// Hide a contract from custom index range queries. Effectively, delist it. All
/// tags and relations remain unaffected.
pub fn exec_migrate(
    ctx: Context,
    params: MigrationParams,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;

    // TODO: refactor this so that it only initializes the migration system
    // TODO: add a separate execution for iteratively running init'ed migration
    // TODO: add a separate execution to retry failed migrations if necessary

    ensure_is_manager(deps.storage, &info.sender)?;

    let mut migration = MIGRATIONS
        .load(deps.storage, &params.name)
        .unwrap_or_else(|_| Migration {
            name: params.name.to_owned(),
            status: MigrationStatus::Running,
            abort_on_error: params.abort_on_error,
            cursor: None,
            n_error: 0,
            n_success: 0,
        });

    if migration.n_error == 0 && migration.n_success == 0 {
        MIGRATION_PARAMS.save(deps.storage, &params.name, &params)?;
    }

    let min_bound = if let Some(cursor) = migration.cursor {
        Some(Bound::Exclusive((cursor, PhantomData)))
    } else {
        None
    };

    let limit = params
        .limit
        .and_then(|x| Some(x as usize))
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(0, MAX_LIMIT);

    let entries: Vec<StdResult<_>> = CONTRACT_ID_2_ADDR
        .range(deps.storage, min_bound, None, Order::Ascending)
        .take(limit)
        .collect();

    let mut migrate_submsgs: Vec<SubMsg> = Vec::with_capacity(entries.len());
    let mut next_cursor_id: Option<u32> = None;

    if entries.is_empty() {
        migration.status = if migration.n_error > 0 {
            MigrationStatus::CompletedWithErrors
        } else {
            MigrationStatus::Completed
        };
    }

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
                msg: params.msg.to_owned(),
            },
            reply_id,
        ))
    }

    migration.cursor = next_cursor_id;

    MIGRATIONS.save(deps.storage, &params.name, &migration)?;

    Ok(Response::new()
        .add_attributes(vec![attr("action", "migrate")])
        .add_submessages(migrate_submsgs))
}

pub fn handle_migration_reply(
    deps: DepsMut,
    reply: Reply,
) -> Result<Response, ContractError> {
    let (migration_name, contract_id) = MIGRATION_REPLY_ID_2_STATE.load(deps.storage, reply.id)?;
    let contract_addr = CONTRACT_ID_2_ADDR.load(deps.storage, contract_id)?;

    let mut migration = MIGRATIONS.load(deps.storage, &migration_name)?;
    let mut resp = Response::new();

    match reply.result {
        SubMsgResult::Ok(_) => {
            migration.n_success = add_u32(migration.n_success, 1)?;
            resp = resp.add_event(Event::new("migration-success").add_attributes(vec![
                attr("migrated_contract_addr", contract_addr.to_string()),
                attr("migration_name", migration_name),
            ]));
        },
        SubMsgResult::Err(e) => {
            if migration.abort_on_error {
                return Err(ContractError::Std(StdError::generic_err(e.to_string())));
            }

            migration.n_error = add_u32(migration.n_error, 1)?;
            if migration.status == MigrationStatus::Completed {
                migration.status = MigrationStatus::CompletedWithErrors;
            }

            MIGRATION_ERRORS.save(
                deps.storage,
                (&migration_name, contract_id),
                &MigrationError {
                    contract: contract_addr.to_owned(),
                    error: e.to_string(),
                },
            )?;

            resp = resp.add_event(Event::new("migration-error").add_attributes(vec![
                attr("migrated_contract_addr", contract_addr.to_string()),
                attr("migration_name", migration_name),
            ]));
        },
    };

    Ok(resp)
}
