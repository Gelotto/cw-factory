//! Range queries with cursor-based pagination.
//!
//! ## Pagination Strategy
//!
//! Queries return a cursor `(value_bytes, contract_id)` when the result set is full (length == limit).
//! The cursor represents the last item returned and is used as an exclusive bound for the next page.
//!
//! ## Cursor Precedence
//!
//! If a cursor is provided, it takes precedence over the `start` bound parameter. This ensures
//! consistent pagination - the cursor represents the exact continuation point from the previous query.
//!
//! ## Descending Order Handling
//!
//! For descending queries, bounds are flipped:
//! - `from_bound` becomes `max_bound` (upper limit)
//! - `to_bound` becomes `min_bound` (lower limit)
//!
//! ## ContractId Tie-Breakers
//!
//! Since index values may not be unique across contracts, we use contract_id as a tie-breaker
//! in composite keys. For bounds that don't specify a contract_id:
//! - Ascending: Use `ContractId::MIN` for start, `ContractId::MAX` for stop
//! - Descending: Use `ContractId::MAX` for start, `ContractId::MIN` for stop
//!
//! This ensures we include all contracts with the boundary value.
//!
//! ## String Key Padding
//!
//! Custom indices and tags use 128-byte padded strings for lexicographic ordering.
//! Cursors strip this padding before returning to clients to reduce payload size.

use std::marker::PhantomData;

use cosmwasm_std::{
    Addr,
    Order,
    Storage,
};
use cw_storage_plus::{
    Bound,
    Map,
};

use crate::{
    error::ContractError,
    msg::{
        ContractsByIndexResponse,
        ContractsInRangeQueryParams,
        IndexRangeBound,
        IndexSelector,
        IndexValue,
    },
    query::ReadonlyContext,
    state::{
        build_index_storage_key,
        storage::{
            ContractId,
            IndexMap,
            CONTRACT_ID_2_ADDR,
            IX_ADMIN,
            IX_CODE_ID,
            IX_CREATED_AT,
            IX_CREATED_BY,
            IX_TAG,
            IX_UPDATED_AT,
        },
    },
};

/// Default number of results per page

const DEFAULT_LIMIT: usize = 100;

/// Maximum number of results per page (prevents excessive gas usage)

const MAX_LIMIT: usize = 500;

/// Query contracts by index range with cursor-based pagination.
///
/// ## Parameters
///
/// - `index`: Which index to query (built-in or custom)
/// - `start`/`stop`: Optional range bounds (Inclusive or Exclusive)
/// - `cursor`: Opaque continuation token from previous query
/// - `limit`: Number of results (clamped to 1-500, default 100)
/// - `desc`: Query in descending order
///
/// ## Returns
///
/// - `addresses`: List of matching contract addresses
/// - `cursor`: Continuation token if more results exist (None if this is the last page)
///
/// ## Cursor Usage
///
/// If cursor is provided, it overrides the `start` bound. This ensures deterministic
/// pagination even if new contracts are created between queries.

pub fn query_contracts_in_range(
    ctx: ReadonlyContext,
    params: ContractsInRangeQueryParams,
) -> Result<ContractsByIndexResponse, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    // Normalize limit within acceptable range (1-500, default 100)
    let limit = params
        .limit
        .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT,),),)
        .unwrap_or(DEFAULT_LIMIT,);

    // Scan the index to get contract IDs matching the query
    let (contract_ids, cursor,) = scan_index(deps.storage, &params, limit,)?;

    // Convert contract IDs to addresses
    let mut contract_addrs: Vec<Addr,> = Vec::with_capacity(contract_ids.len(),);

    for contract_id in contract_ids.iter() {

        contract_addrs.push(CONTRACT_ID_2_ADDR.load(deps.storage, *contract_id,)?,);
    }

    Ok(ContractsByIndexResponse {
        addresses: contract_addrs,
        cursor,
    },)
}

/// Scan an index and return matching contract IDs with pagination cursor.
///
/// ## Cursor vs Start Bound
///
/// Cursor takes precedence over start bound. If a cursor is provided, it's used as an
/// exclusive starting point, ignoring the `start` parameter. This prevents issues when
/// new contracts are created between paginated queries.
///
/// ## Padding Handling
///
/// Custom indices and tags use 128-byte padded strings in storage for lexicographic ordering.
/// This function handles padding/stripping transparently:
/// - Cursor bytes are padded when creating bounds
/// - Returned cursor bytes are stripped to reduce payload size
///
/// ## Descending Order
///
/// For descending queries, from_bound and to_bound are flipped to become max_bound and min_bound.

fn scan_index(
    store: &dyn Storage,
    params: &ContractsInRangeQueryParams,
    limit: usize,
) -> Result<(Vec<ContractId,>, Option<(Vec<u8,>, ContractId,),>,), ContractError,> {

    let desc = params.desc.unwrap_or_default();

    // Heap-allocated storage for byte arrays (needed for lifetime management in bound references)
    let mut custom_index_storage_key: Box<String,> = Box::new(String::new(),);

    let mut start_bytes: Box<Vec<u8,>,> = Box::new(vec![],);

    let mut stop_bytes: Box<Vec<u8,>,> = Box::new(vec![],);

    // Select which index to query (built-in or dynamic custom index)
    let map = match &params.index {
        IndexSelector::CreatedBy => IX_CREATED_BY,
        IndexSelector::CreatedAt => IX_CREATED_AT,
        IndexSelector::UpdatedAt => IX_UPDATED_AT,
        IndexSelector::CodeId => IX_CODE_ID,
        IndexSelector::Admin => IX_ADMIN,
        IndexSelector::Tag => IX_TAG,
        IndexSelector::Custom(index_name,) => {

            // Build dynamic storage key for custom index
            *custom_index_storage_key = build_index_storage_key(&index_name,);

            let map: IndexMap = Map::new(custom_index_storage_key.as_str(),);

            map
        },
    };

    // Build "from" bound: cursor takes precedence over start parameter
    let from_bound = match &params.cursor {
        Some((bytes, id,),) => {

            // Cursor provided: use it as exclusive starting point (ignoring start parameter)
            // Pad cursor bytes for custom indices and tags to match storage format
            *start_bytes = match &params.index {
                IndexSelector::Custom(_,) | IndexSelector::Tag => IndexValue::pad(bytes.to_owned(),),
                _ => bytes.to_owned(),
            };

            Some(Bound::Exclusive(((start_bytes.as_slice(), *id,), PhantomData,),),)
        },
        None => {

            // No cursor: use start bound if provided
            // Use ContractId::MIN/MAX as tie-breaker to include all contracts with boundary value
            let id = if desc { ContractId::MAX } else { ContractId::MIN };

            match &params.start {
                Some(bound,) => match bound {
                    IndexRangeBound::Exclusive(value,) => {

                        *start_bytes = value.to_bytes();

                        Some(Bound::Exclusive(((start_bytes.as_slice(), id,), PhantomData,),),)
                    },
                    IndexRangeBound::Inclusive(value,) => {

                        *start_bytes = value.to_bytes();

                        Some(Bound::Inclusive(((start_bytes.as_slice(), id,), PhantomData,),),)
                    },
                },
                None => None,
            }
        },
    };

    // Build "to" bound from stop parameter (cursor only affects from_bound)
    let to_bound = match &params.stop {
        Some(bound,) => {

            // Use opposite ContractId extreme for stop bound
            let id = if desc { ContractId::MIN } else { ContractId::MAX };

            Some(match bound {
                IndexRangeBound::Exclusive(value,) => {

                    *stop_bytes = value.to_bytes();

                    Bound::Exclusive(((stop_bytes.as_slice(), id,), PhantomData,),)
                },
                IndexRangeBound::Inclusive(value,) => {

                    *stop_bytes = value.to_bytes();

                    Bound::Inclusive(((stop_bytes.as_slice(), id,), PhantomData,),)
                },
            },)
        },
        None => None,
    };

    // For descending order, flip bounds: from_bound → max, to_bound → min
    let (min_bound, max_bound, order,) = if desc {

        (to_bound, from_bound, Order::Descending,)
    } else {

        (from_bound, to_bound, Order::Ascending,)
    };

    // Execute range query and collect keys up to limit
    let keys: Vec<_,> = map
        .keys(store, min_bound, max_bound, order,)
        .take(limit,)
        .map(|r| r.unwrap(),)
        .collect();

    // Extract contract IDs from composite keys
    let contract_ids: Vec<ContractId,> = keys.iter().map(|k| k.1,).collect();

    // Build cursor ONLY if we hit the limit (indicating more results may exist)
    // If fewer than limit results were returned, we've reached the end
    let cursor = if keys.len() == limit {

        keys.last().and_then(|(a, b,)| {

            // Strip padding from custom index and tag bytes before returning
            // This reduces payload size while preserving pagination correctness
            let bytes = match &params.index {
                IndexSelector::Custom(_,) | IndexSelector::Tag => IndexValue::strip(a.to_vec(),),
                _ => a.to_vec(),
            };

            Some((bytes, *b,),)
        },)
    } else {

        // Result set is smaller than limit: this is the last page
        None
    };

    Ok((contract_ids, cursor,),)
}
