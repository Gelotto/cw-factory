use cosmwasm_std::Order;

use crate::{
    error::ContractError,
    msg::MigrationSessionResponse,
    query::ReadonlyContext,
    state::{
        models::{Migration, MigrationError},
        storage::{MIGRATIONS, MIGRATION_ERRORS},
    },
};

pub fn query_migration_session(
    ctx: ReadonlyContext,
    session_name: String,
) -> Result<MigrationSessionResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;

    let Migration {
        params,
        status,
        cursor,
        retry_cursor,
        n_success,
        n_error,
    } = MIGRATIONS.load(deps.storage, &session_name)?;

    let errors: Vec<MigrationError> = MIGRATION_ERRORS
        .prefix(&session_name)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.unwrap().1)
        .collect();

    Ok(MigrationSessionResponse {
        cursor,
        params,
        status,
        retry_cursor,
        n_error,
        n_success,
        errors,
    })
}
