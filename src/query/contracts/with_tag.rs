//! Weighted tag queries with weight range filtering.
//!
//! ## Index Structure
//!
//! The weighted tag index uses a triple-key composite:
//! `(tag_bytes, weight, contract_id) -> marker`
//!
//! This structure enables:
//! - Filtering contracts by tag name (exact match on tag_bytes)
//! - Sorting by weight within a tag (primary sort key after tag)
//! - Weight range filtering (min_weight, max_weight bounds)
//! - Deterministic ordering with contract_id as final tie-breaker
//!
//! ## Weight Range Filtering
//!
//! Both min_weight and max_weight support Inclusive/Exclusive bounds:
//! - `min_weight: Inclusive(50)` → weight >= 50
//! - `max_weight: Exclusive(100)` → weight < 100
//! - Combined: `50 <= weight < 100`
//!
//! ## Cursor Format
//!
//! Cursors contain all three key components: `(tag_bytes, weight, contract_id)`.
//! This preserves exact position in the weighted tag index for pagination.
//!
//! ## Padding
//!
//! Tag strings are padded to 128 bytes in storage. The cursor returns stripped
//! bytes but automatically re-pads when used to resume iteration.

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
        ContractsByTagQueryParams,
        ContractsByTagResponse,
        IndexValue,
        TagWeightRangeBound,
    },
    query::ReadonlyContext,
    state::storage::{
        ContractId,
        CONTRACT_ID_2_ADDR,
        IX_WEIGHTED_TAG,
    },
};

/// Default number of results per page

const DEFAULT_LIMIT: usize = 100;

/// Maximum number of results per page (prevents excessive gas usage)

const MAX_LIMIT: usize = 500;

/// Query contracts by tag with optional weight range filtering.
///
/// ## Parameters
///
/// - `tag`: Tag name to filter by (exact match)
/// - `min_weight`/`max_weight`: Optional weight bounds (Inclusive or Exclusive)
/// - `cursor`: Opaque continuation token from previous query
/// - `limit`: Number of results (clamped to 1-500, default 100)
/// - `desc`: Sort by weight descending (highest weights first)
///
/// ## Returns
///
/// - `addresses`: List of matching contract addresses
/// - `weights`: Corresponding weight for each contract (parallel array)
/// - `cursor`: Continuation token if more results exist
///
/// ## Use Cases
///
/// - Priority queues: Query by tag with desc=true to get highest-priority contracts
/// - Tier filtering: Use weight bounds to filter by membership tier (e.g., 80-100 = premium)
/// - Ranking: Weights represent scores, ratings, or importance levels

pub fn query_contracts_with_tag(
    ctx: ReadonlyContext,
    params: ContractsByTagQueryParams,
) -> Result<ContractsByTagResponse, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    // Normalize limit within acceptable range (1-500, default 100)
    let limit = params
        .limit
        .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT,),),)
        .unwrap_or(DEFAULT_LIMIT,);

    // Scan weighted tag index to get contract IDs and their weights
    let (contract_id_weights, cursor,) = scan_tag(deps.storage, &params, limit,)?;

    // Convert contract IDs to addresses and extract weights into parallel arrays
    let mut addresses: Vec<Addr,> = Vec::with_capacity(contract_id_weights.len(),);

    let mut weights: Vec<u16,> = Vec::with_capacity(contract_id_weights.len(),);

    for (contract_id, weight,) in contract_id_weights.iter() {

        addresses.push(CONTRACT_ID_2_ADDR.load(deps.storage, *contract_id,)?,);

        weights.push(*weight,);
    }

    Ok(ContractsByTagResponse {
        addresses,
        weights,
        cursor,
    },)
}

/// Scan the weighted tag index and return matching contract IDs with weights.
///
/// ## Cursor Handling
///
/// The cursor contains (tag_bytes, weight, contract_id) and is used as an exclusive
/// starting point. Cursor bytes are already padded from storage, so they're used directly.
///
/// ## Weight Bounds
///
/// When no cursor is provided, min_weight and max_weight define the weight range.
/// The tag_bytes remain constant across all queries for the same tag.

fn scan_tag(
    store: &dyn Storage,
    params: &ContractsByTagQueryParams,
    limit: usize,
) -> Result<(Vec<(ContractId, u16,),>, Option<(Vec<u8,>, u16, ContractId,),>,), ContractError,> {

    // Convert tag string to padded bytes (128 bytes for lexicographic ordering)
    let bytes_vec = IndexValue::String(params.tag.clone(),).to_bytes();

    let bytes = bytes_vec.as_slice();

    let desc = params.desc.unwrap_or_default();

    // Build "from" bound: cursor takes precedence over min_weight parameter
    let from_bound = match &params.cursor {
        Some((cursor_bytes, w, id,),) => {

            // Cursor provided: resume from exact position (tag_bytes, weight, contract_id)
            // Cursor bytes are already padded, use as-is
            Some(Bound::Exclusive(((cursor_bytes.as_slice(), *w, *id,), PhantomData,),),)
        },
        None => {

            // No cursor: use min_weight bound if provided
            // Use ContractId::MIN/MAX as tie-breaker to include all contracts at boundary weight
            let id = if desc { ContractId::MAX } else { ContractId::MIN };

            match &params.min_weight {
                Some(bound,) => match bound {
                    TagWeightRangeBound::Exclusive(weight,) => {
                        Some(Bound::Exclusive(((bytes, *weight, id,), PhantomData,),),)
                    },
                    TagWeightRangeBound::Inclusive(weight,) => {
                        Some(Bound::Inclusive(((bytes, *weight, id,), PhantomData,),),)
                    },
                },
                None => None,
            }
        },
    };

    // Build "to" bound from max_weight parameter (cursor only affects from_bound)
    let to_bound = match &params.max_weight {
        Some(bound,) => {

            // Use opposite ContractId extreme for stop bound
            let id = if desc { ContractId::MIN } else { ContractId::MAX };

            Some(match bound {
                &TagWeightRangeBound::Exclusive(w,) => Bound::Exclusive(((bytes, w, id,), PhantomData,),),
                &TagWeightRangeBound::Inclusive(w,) => Bound::Inclusive(((bytes, w, id,), PhantomData,),),
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

    // Execute range query on IX_WEIGHTED_TAG: (tag_bytes, weight, contract_id) -> marker
    // Results are ordered by weight (then contract_id for ties)
    let keys: Vec<_,> = IX_WEIGHTED_TAG
        .keys(store, min_bound, max_bound, order,)
        .take(limit,)
        .map(|r| r.unwrap(),)
        .collect();

    // Extract (contract_id, weight) pairs from triple keys
    // Key structure: (tag_bytes, weight, contract_id) → extract (contract_id, weight)
    let contract_ids: Vec<(ContractId, u16,),> = keys.iter().map(|k| (k.2, k.1,),).collect();

    // Build cursor ONLY if we hit the limit (indicating more results may exist)
    let cursor = if keys.len() == limit {

        // Return all three components: (tag_bytes, weight, contract_id)
        // Tag bytes are stripped before returning to reduce payload size
        keys.last()
            .and_then(|(a, b, c,)| Some((IndexValue::strip(a.to_vec(),), *b, *c,),),)
    } else {

        // Result set is smaller than limit: this is the last page
        None
    };

    Ok((contract_ids, cursor,),)
}
