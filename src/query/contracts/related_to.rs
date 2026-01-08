//! Relationship graph queries (reverse direction: who relates to this address?).
//!
//! ## Bidirectional Relationship Storage
//!
//! Relationships are stored in two indices for efficient bidirectional traversal:
//! - `IX_REL_CONTRACT_ADDR`: Forward (contract_id, edge, addr) → value
//! - `IX_REL_ADDR`: Reverse (addr, edge, contract_id) → marker
//!
//! This query uses the reverse index to find all contracts that have a relationship
//! TO a specific address.
//!
//! ## Edge Encoding
//!
//! Edges are composite keys built from relationship name and optional value:
//! `edge_bytes = name_bytes || value_bytes` (concatenation)
//!
//! This enables range queries like:
//! - "All 'depends_on' relationships" (name-only)
//! - "All 'version:1.0' relationships" (name + value)
//! - "All 'priority:50-100' relationships" (name + value range)
//!
//! ## NameValue Bounds
//!
//! Start/stop bounds use `NameValue` type which can specify:
//! - Name only: Match all relationships with this name (any value)
//! - Name + Value: Match specific name-value combination
//! - Value ranges: Specify start/stop values for range filtering
//!
//! ## Value Lookup
//!
//! The reverse index (`IX_REL_ADDR`) only stores markers, not values.
//! Relationship values are retrieved by cross-referencing the forward index
//! (`IX_REL_CONTRACT_ADDR`) using the (contract_id, edge, addr) key.

use std::marker::PhantomData;

use cosmwasm_std::{
    Addr,
    Order,
    Storage,
};
use cw_storage_plus::Bound;

use crate::{
    error::ContractError,
    msg::{
        ContractsRelatedToParams,
        ContractsRelatedToResponse,
        IndexValue,
        RangeQueryBound,
    },
    query::ReadonlyContext,
    state::storage::{
        ContractId,
        CONTRACT_ID_2_ADDR,
        IX_REL_ADDR,
        IX_REL_CONTRACT_ADDR,
    },
};

/// Default number of results per page

const DEFAULT_LIMIT: usize = 100;

/// Maximum number of results per page (prevents excessive gas usage)

const MAX_LIMIT: usize = 500;

/// Query contracts that have a relationship TO the specified address.
///
/// ## Parameters
///
/// - `address`: Target address to find relationships pointing to
/// - `start`/`stop`: Optional NameValue bounds for edge filtering
/// - `cursor`: Opaque continuation token from previous query
/// - `limit`: Number of results (clamped to 1-500, default 100)
/// - `desc`: Query in descending order
///
/// ## Returns
///
/// - `addresses`: List of contract addresses that relate to the target address
/// - `values`: Corresponding relationship values (parallel array, may be None)
/// - `cursor`: Continuation token if more results exist
///
/// ## Use Cases
///
/// - Dependency tracking: "Which contracts depend on this library?"
/// - Permission graphs: "Which contracts have access to this resource?"
/// - Social graphs: "Which users follow this account?"
/// - Version tracking: "Which contracts reference version X of this dependency?"

pub fn query_contracts_related_to(
    ctx: ReadonlyContext,
    params: ContractsRelatedToParams,
) -> Result<ContractsRelatedToResponse, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    // Normalize limit within acceptable range (1-500, default 100)
    let limit = params
        .limit
        .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT,),),)
        .unwrap_or(DEFAULT_LIMIT,);

    // Scan reverse relationship index to find contracts pointing to this address
    let (ids_and_rel_values, cursor,) = scan_relations(deps.storage, &params, limit,)?;

    // Convert contract IDs to addresses and extract values into parallel arrays
    let mut addresses: Vec<Addr,> = Vec::with_capacity(ids_and_rel_values.len(),);

    let mut values: Vec<Option<IndexValue,>,> = Vec::with_capacity(ids_and_rel_values.len(),);

    for (contract_id, value,) in ids_and_rel_values.iter() {

        addresses.push(CONTRACT_ID_2_ADDR.load(deps.storage, *contract_id,)?,);

        values.push(value.to_owned(),);
    }

    Ok(ContractsRelatedToResponse {
        addresses,
        values,
        cursor,
    },)
}

/// Scan the reverse relationship index and return matching contract IDs with values.
///
/// ## Edge Byte Construction
///
/// NameValue bounds are converted to edge bytes via `as_edge_bytes()`:
/// - Name only: `name_bytes` (matches all values for this name)
/// - Name + Value: `name_bytes || value_bytes` (exact match or range start/stop)
///
/// ## Value Retrieval
///
/// The reverse index (`IX_REL_ADDR`) only contains markers, not values.
/// For each result, we cross-reference the forward index (`IX_REL_CONTRACT_ADDR`)
/// to retrieve the actual relationship value.

fn scan_relations(
    store: &dyn Storage,
    params: &ContractsRelatedToParams,
    limit: usize,
) -> Result<(Vec<(ContractId, Option<IndexValue,>,),>, Option<ContractId,>,), ContractError,> {

    let desc = params.desc.unwrap_or_default();

    // Heap-allocated storage for edge bytes (needed for lifetime management in bound references)
    let mut from_edge_box: Box<Vec<u8,>,> = Box::new(vec![],);

    // Build "from" bound: cursor takes precedence over start parameter
    let from_bound = match &params.cursor {
        Some((id, edge,),) => {

            // Cursor provided: resume from exact position (addr, edge, contract_id)
            *from_edge_box = edge.clone();

            Some(Bound::Exclusive((
                (params.address.as_bytes(), from_edge_box.as_slice(), *id,),
                PhantomData,
            ),),)
        },
        None => {

            // No cursor: use start bound if provided
            // Use ContractId::MIN/MAX as tie-breaker to include all contracts with boundary edge
            let id = if desc { ContractId::MAX } else { ContractId::MIN };

            match &params.start {
                Some(bound,) => match &bound {
                    RangeQueryBound::Inclusive(nv,) => {

                        // Convert NameValue to edge bytes (name || value)
                        *from_edge_box = nv.as_edge_bytes();

                        Some(Bound::Inclusive((
                            (params.address.as_bytes(), from_edge_box.as_slice(), id,),
                            PhantomData,
                        ),),)
                    },
                    RangeQueryBound::Exclusive(nv,) => {

                        *from_edge_box = nv.as_edge_bytes();

                        Some(Bound::Exclusive((
                            (params.address.as_bytes(), from_edge_box.as_slice(), id,),
                            PhantomData,
                        ),),)
                    },
                },
                None => None,
            }
        },
    };

    // Heap-allocated storage for stop edge bytes
    let mut to_edge_box: Box<Vec<u8,>,> = Box::new(vec![],);

    // Build "to" bound from stop parameter (cursor only affects from_bound)
    let to_bound = {

        // Use opposite ContractId extreme for stop bound
        let id = if desc { ContractId::MIN } else { ContractId::MAX };

        match &params.stop {
            Some(bound,) => match &bound {
                RangeQueryBound::Inclusive(nv,) => {

                    *to_edge_box = nv.as_edge_bytes();

                    Some(Bound::Inclusive((
                        (params.address.as_bytes(), to_edge_box.as_slice(), id,),
                        PhantomData,
                    ),),)
                },
                RangeQueryBound::Exclusive(nv,) => {

                    *to_edge_box = nv.as_edge_bytes();

                    Some(Bound::Exclusive((
                        (params.address.as_bytes(), to_edge_box.as_slice(), id,),
                        PhantomData,
                    ),),)
                },
            },
            None => None,
        }
    };

    // For descending order, flip bounds: from_bound → max, to_bound → min
    let (min_bound, max_bound, order,) = if desc {

        (to_bound, from_bound, Order::Descending,)
    } else {

        (from_bound, to_bound, Order::Ascending,)
    };

    // Execute range query on IX_REL_ADDR: (addr, edge, contract_id) → marker
    // This finds all contracts that have a relationship TO the specified address
    let keys: Vec<_,> = IX_REL_ADDR
        .keys(store, min_bound, max_bound, order,)
        .map(|r| r.unwrap(),)
        .take(limit,)
        .collect();

    let mut contract_ids_and_values: Vec<(ContractId, Option<IndexValue,>,),> = Vec::with_capacity(keys.len(),);

    // For each relationship, cross-reference the forward index to get the value
    // Reverse index only contains markers; forward index stores actual values
    for (addr, edge, contract_id,) in keys.iter() {

        // Load relationship value from forward index: (contract_id, edge, addr) → value
        let value = IX_REL_CONTRACT_ADDR.load(store, (*contract_id, edge, &addr,),)?;

        contract_ids_and_values.push((*contract_id, value,),);
    }

    // Build cursor ONLY if we hit the limit (indicating more results may exist)
    // Cursor is just the contract_id since edge is implicit from the relationship key
    let cursor = if contract_ids_and_values.len() == limit {

        contract_ids_and_values.last().and_then(|(id, _,)| Some(*id,),)
    } else {

        // Result set is smaller than limit: this is the last page
        None
    };

    Ok((contract_ids_and_values, cursor,),)
}
