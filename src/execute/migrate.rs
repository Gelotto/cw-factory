//! Batch migration operations with stateful sessions.
//!
//! ## Migration Session Lifecycle
//!
//! 1. **Begin**: Create named session with parameters (target code_id, batch size, error strategy)
//! 2. **Step**: Process next batch of contracts, save cursor, emit submessages
//! 3. **Reply Handling**: Track success/failure for each contract in batch
//! 4. **Retry**: Re-attempt failed contracts (if error_strategy = Retry)
//! 5. **Cancel**: Clean up session and error tracking
//!
//! ## Cursor-Based Batching
//!
//! Large migrations are split into steps to avoid gas limits. Each step:
//! - Loads `batch_size` contracts starting from saved cursor
//! - Emits migration submessages with reply handlers
//! - Saves next cursor if more contracts remain
//! - Marks session Complete if all contracts processed
//!
//! ## Error Handling Strategies
//!
//! - **Abort**: First failure rolls back entire transaction (atomic migration)
//! - **Retry**: Failures tracked in MIGRATION_ERRORS, session continues, can retry later
//!
//! ## Why Named Sessions?
//!
//! Named sessions allow multiple concurrent migrations (e.g., "upgrade-v2" and "fix-bug")
//! and provide clear tracking in queries.

use std::marker::PhantomData;

use crate::{
    error::ContractError,
    math::{
        add_u32,
        add_u64,
        sub_u32,
    },
    msg::{
        MigrationParams,
        SingletonMigrationParams,
    },
    state::{
        models::{
            Migration,
            MigrationError,
            MigrationErrorStrategy,
            MigrationStatus,
        },
        storage::{
            CONTRACT_ADDR_2_ID,
            CONTRACT_ID_2_ADDR,
            ID_2_CODE_ID,
            MIGRATIONS,
            MIGRATION_ERRORS,
            MIGRATION_REPLY_ID_2_STATE,
            REPLY_ID_COUNTER,
        },
    },
};
use cosmwasm_std::{
    attr,
    to_json_binary,
    DepsMut,
    Empty,
    Event,
    Order,
    Reply,
    Response,
    StdError,
    StdResult,
    SubMsg,
    SubMsgResult,
    WasmMsg,
};
use cw_storage_plus::Bound;

use super::Context;

/// Default batch size for migration sessions

const DEFAULT_LIMIT: u16 = 50;

/// Maximum batch size for migration sessions (to prevent gas limit issues)

const MAX_LIMIT: u16 = 100;

/// Migrate a single contract.
///
/// Validates the contract against `from_code_id` filter if specified, then
/// emits a migration submessage. Used for one-off migrations outside of
/// batch sessions.

pub fn exec_migrate_one(
    ctx: Context,
    params: SingletonMigrationParams,
) -> Result<Response, ContractError,> {

    let Context { deps, .. } = ctx;

    let mut resp = Response::new().add_attributes(vec![attr("action", "migrate",)],);

    let addr = deps.api.addr_validate(params.contract.as_str(),)?;

    let id = CONTRACT_ADDR_2_ID.load(deps.storage, &addr,)?;

    // Filter by source code_id if specified (only migrate specific version)
    if let Some(from_code_id,) = params.from_code_id {

        let code_id = ID_2_CODE_ID.load(deps.storage, id,)?;

        if u64::from_le_bytes(code_id.as_slice().try_into().unwrap(),) != from_code_id.u64() {

            // Contract doesn't match source code_id, skip migration
            return Ok(resp,);
        }
    }

    resp = resp.add_submessage(SubMsg::new(WasmMsg::Migrate {
        contract_addr: addr.to_string(),
        new_code_id: params.to_code_id.u64(),
        msg: params
            .migrate_msg
            .to_owned()
            .unwrap_or_else(|| to_json_binary(&Empty {},).unwrap(),),
    },),);

    Ok(resp,)
}

/// Begin a batch migration session.
///
/// Creates a new named migration session with normalized parameters.
/// The session starts in Running status with cursor at None (beginning).

pub fn exec_begin_migration(
    ctx: Context,
    params: MigrationParams,
) -> Result<Response, ContractError,> {

    let Context { deps, .. } = ctx;

    let mut params = params;

    // Normalize batch size: use default if not specified, clamp to valid range
    params.batch_size = Some(params.batch_size.unwrap_or(DEFAULT_LIMIT,).clamp(0, MAX_LIMIT,),);

    MIGRATIONS.save(
        deps.storage,
        &params.name,
        &Migration {
            params: params.to_owned(),
            status: MigrationStatus::Running,
            cursor: None,       // Start from beginning
            retry_cursor: None, // No retry cursor yet
            n_error: 0,
            n_success: 0,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "begin_migration",),
        attr("session_name", params.name.to_owned(),),
    ],),)
}

pub fn exec_step_migration(
    ctx: Context,
    session_name: String,
) -> Result<Response, ContractError,> {

    let Context { deps, .. } = ctx;

    let mut migration = MIGRATIONS.load(deps.storage, &session_name,)?;

    if migration.status == MigrationStatus::Complete {

        return Err(ContractError::MigrationComplete { name: session_name, },);
    }

    let params = &migration.params;

    let batch_size = params.batch_size.unwrap();

    // Resume from cursor if this is a continuation of a previous step.
    // Use Exclusive bound so we don't re-process the last contract from previous batch.
    let min_bound = if let Some(cursor,) = migration.cursor {

        Some(Bound::Exclusive((cursor, PhantomData,),),)
    } else {

        // First batch: start from beginning
        None
    };

    // Load next batch of contract ID/Address pairs to process
    let entries: Vec<StdResult<_,>,> = CONTRACT_ID_2_ADDR
        .range(deps.storage, min_bound, None, Order::Ascending,)
        .take(batch_size as usize,)
        .collect();

    let mut migrate_submsgs: Vec<SubMsg,> = Vec::with_capacity(entries.len(),);

    // Track the last ID we process to use as next cursor
    let mut next_cursor_id: Option<u32,> = None;

    for result in entries {

        let (id, addr,) = result?;

        // Update cursor to this contract ID
        next_cursor_id = Some(id,);

        // Filter by source code_id if specified (only migrate specific version)
        if let Some(from_code_id,) = params.from_code_id {

            let code_id = ID_2_CODE_ID.load(deps.storage, id,)?;

            if u64::from_le_bytes(code_id.as_slice().try_into().unwrap(),) != from_code_id.u64() {

                // Skip contracts that don't match the source code_id filter
                continue;
            }
        }

        // Generate unique reply ID for this migration submessage
        let reply_id = REPLY_ID_COUNTER
            .update(deps.storage, |n| -> Result<_, ContractError,> {

                add_u64(n, 1u64,)
            },)?
            .u64()
            - 1;

        // Store context for reply handler: (session_name, contract_id)
        // This enables the reply handler to route back to the correct session
        MIGRATION_REPLY_ID_2_STATE.save(deps.storage, reply_id, &(params.name.to_owned(), id,),)?;

        // Emit migration submessage with reply_always to track both success and failure
        migrate_submsgs.push(SubMsg::reply_always(
            WasmMsg::Migrate {
                contract_addr: addr.to_string(),
                new_code_id: params.to_code_id.u64(),
                msg: params
                    .migrate_msg
                    .to_owned()
                    .unwrap_or_else(|| to_json_binary(&Empty {},).unwrap(),),
            },
            reply_id,
        ),)
    }

    // Mark session complete if we didn't fill the batch (no more contracts to process)
    if migrate_submsgs.len() < batch_size as usize {

        migration.status = MigrationStatus::Complete;

        migration.cursor = None;
    } else {

        // More contracts remain: save cursor for next step
        migration.cursor = next_cursor_id;
    }

    MIGRATIONS.save(deps.storage, &params.name, &migration,)?;

    Ok(Response::new().add_submessages(migrate_submsgs,).add_attributes(vec![
        attr("action", "step_migration",),
        attr("session_name", session_name.to_owned(),),
    ],),)
}

/// Retry failed migrations from a session.
///
/// Iterates through MIGRATION_ERRORS for this session and re-attempts migration.
/// Optionally accepts updated migration parameters (e.g., new migrate_msg) while
/// preserving the session name.

pub fn exec_retry_migration(
    ctx: Context,
    session_name: String,
    override_migration_params: Option<MigrationParams,>,
) -> Result<Response, ContractError,> {

    let Context { deps, .. } = ctx;

    let mut migration = MIGRATIONS.load(deps.storage, &session_name,)?;

    let mut migrate_submsgs: Vec<SubMsg,> = Vec::with_capacity(migration.params.batch_size.unwrap() as usize,);

    let mut next_cursor_id: Option<u32,> = None;

    // Allow updating migration parameters on retry (e.g., different migrate_msg)
    // This is useful if the original migration failed due to a bug in the migrate message
    if let Some(mut override_params,) = override_migration_params {

        // Preserve the session name to prevent state corruption
        override_params.name = migration.params.name;

        // Normalize batch size
        override_params.batch_size = Some(
            override_params
                .batch_size
                .unwrap_or(DEFAULT_LIMIT,)
                .clamp(0, MAX_LIMIT,),
        );

        migration.params = override_params;
    }

    let params = &migration.params;

    // Resume from retry_cursor if this is a continuation
    let min_bound = if let Some(cursor,) = migration.retry_cursor {

        Some(Bound::Exclusive((cursor, PhantomData,),),)
    } else {

        // First retry batch: start from beginning of error list
        None
    };

    // Load next batch of failed contract IDs from MIGRATION_ERRORS
    let entries = MIGRATION_ERRORS
        .prefix(&session_name,)
        .range(deps.storage, min_bound, None, Order::Ascending,)
        .take(migration.params.batch_size.unwrap() as usize,)
        .collect::<Vec<StdResult<_,>,>>();

    // Re-attempt migration for each failed contract
    // No need to re-check from_code_id filter since these contracts already
    // passed that check in the original migration attempt
    for result in entries {

        let (id, error,) = result?;

        next_cursor_id = Some(id,);

        let reply_id = REPLY_ID_COUNTER
            .update(deps.storage, |n| -> Result<_, ContractError,> {

                add_u64(n, 1u64,)
            },)?
            .u64()
            - 1;

        MIGRATION_REPLY_ID_2_STATE.save(deps.storage, reply_id, &(params.name.to_owned(), id,),)?;

        // Remove the error entry now; it will be re-added in the reply
        // handler if it fails again. This prevents duplicate error tracking.
        MIGRATION_ERRORS.remove(deps.storage, (&params.name, id,),);

        migrate_submsgs.push(SubMsg::reply_always(
            WasmMsg::Migrate {
                contract_addr: error.contract.to_string(),
                new_code_id: params.to_code_id.u64(),
                msg: params
                    .migrate_msg
                    .to_owned()
                    .unwrap_or_else(|| to_json_binary(&Empty {},).unwrap(),),
            },
            reply_id,
        ),)
    }

    if migrate_submsgs.len() < params.batch_size.unwrap() as usize {

        migration.status = MigrationStatus::Complete;

        migration.cursor = None;
    } else {

        migration.cursor = next_cursor_id;
    }

    MIGRATIONS.save(deps.storage, &params.name, &migration,)?;

    Ok(Response::new().add_submessages(migrate_submsgs,).add_attributes(vec![
        attr("action", "retry_migration",),
        attr("session_name", session_name.to_owned(),),
    ],),)
}

/// Cancel a migration session and clean up all associated state.
///
/// Removes the session from MIGRATIONS and cleans up all error tracking
/// and reply ID mappings for this session.

pub fn exec_cancel_migration(
    ctx: Context,
    session_name: String,
) -> Result<Response, ContractError,> {

    let Context { deps, .. } = ctx;

    // Remove the migration session
    MIGRATIONS.remove(deps.storage, &session_name,);

    // Clean up all error tracking for this session
    for result in MIGRATION_ERRORS
        .prefix(&session_name,)
        .range(deps.storage, None, None, Order::Ascending,)
        .collect::<Vec<StdResult<_,>,>>()
    {

        let (id, error,) = result?;

        // Remove reply ID mapping to prevent stale references
        MIGRATION_REPLY_ID_2_STATE.remove(deps.storage, error.reply_id.u64(),);

        // Remove error entry
        MIGRATION_ERRORS.remove(deps.storage, (&session_name, id,),);
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "cancel_migration",),
        attr("session_name", session_name.to_owned(),),
    ],),)
}

/// Handle migration submessage replies.
///
/// Updates session state based on success/failure:
/// - **Success**: Increment n_success, remove from error tracking if previously failed
/// - **Failure**:
///   - Abort strategy: Immediately fail the transaction
///   - Retry strategy: Increment n_error, save error details for retry

pub fn handle_migration_reply(
    deps: DepsMut,
    reply: Reply,
) -> Result<Response, ContractError,> {

    // Load session context from reply ID
    let (session_name, contract_id,) = MIGRATION_REPLY_ID_2_STATE.load(deps.storage, reply.id,)?;

    let contract_addr = CONTRACT_ID_2_ADDR.load(deps.storage, contract_id,)?;

    let mut migration = MIGRATIONS.load(deps.storage, &session_name,)?;

    let mut resp = Response::new();

    match reply.result {
        SubMsgResult::Ok(_,) => {

            migration.n_success = add_u32(migration.n_success, 1,)?;

            // If this was a retry of a previously failed migration, remove it from error tracking
            if MIGRATION_ERRORS.has(deps.storage, (&session_name, contract_id,),) {

                MIGRATION_ERRORS.remove(deps.storage, (&session_name, contract_id,),);

                migration.n_error = sub_u32(migration.n_error, 1,)?;
            }

            resp = resp.add_event(Event::new("migration-success",).add_attributes(vec![
                attr("migrated_contract_addr", contract_addr.to_string(),),
                attr("session_name", session_name.to_owned(),),
            ],),);
        },
        SubMsgResult::Err(e,) => {

            // Abort strategy: fail the entire transaction on first error
            if let MigrationErrorStrategy::Abort = migration.params.error_strategy {

                return Err(ContractError::Std(StdError::generic_err(e.to_string(),),),);
            }

            // Retry strategy: track the error and continue with other contracts
            migration.n_error = add_u32(migration.n_error, 1,)?;

            MIGRATION_ERRORS.save(
                deps.storage,
                (&session_name, contract_id,),
                &MigrationError {
                    reply_id: reply.id.into(),
                    contract: contract_addr.to_owned(),
                    error: e.to_string(),
                },
            )?;

            resp = resp.add_event(Event::new("migration-error",).add_attributes(vec![
                attr("migrated_contract_addr", contract_addr.to_string(),),
                attr("session_name", session_name.to_owned(),),
            ],),);
        },
    };

    // Persist updated migration state
    MIGRATIONS.save(deps.storage, &session_name, &migration,)?;

    Ok(resp,)
}
