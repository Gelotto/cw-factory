//! Contract relationship queries (forward direction: who does this contract relate to?).
//!
//! ## Forward vs Reverse Queries
//!
//! This query uses the forward relationship index (`IX_REL_CONTRACT_ADDR`):
//! - **Forward**: "Who does contract X relate to?" (this file)
//! - **Reverse**: "Who relates to address Y?" (contracts/related_to.rs)
//!
//! ## Index Structure
//!
//! `IX_REL_CONTRACT_ADDR`: `(contract_id, edge_bytes, addr_bytes) -> value`
//!
//! Where:
//! - `contract_id`: The contract whose relationships we're querying
//! - `edge_bytes`: Composite key `name_bytes || value_bytes`
//! - `addr_bytes`: Target address of the relationship
//! - `value`: Optional relationship value (stored as value, not in key)
//!
//! ## Edge Deserialization
//!
//! Unlike the reverse query, this query needs to deserialize edge bytes back into:
//! - Relationship name (extracted from first part of edge bytes)
//! - Relationship value (stored separately in index value)
//!
//! The returned `RelatedAddress` struct contains:
//! - `address`: Target address
//! - `name`: Relationship name (deserialized from edge bytes)
//! - `value`: Optional relationship value
//!
//! ## Use Cases
//!
//! - Dependency listing: "What libraries does this contract depend on?"
//! - Permission queries: "What resources does this contract have access to?"
//! - Social graphs: "Who does this user follow?"
//! - Configuration: "What external services does this contract integrate with?"

use std::marker::PhantomData;

use cosmwasm_std::{
    Addr,
    Order,
};
use cw_storage_plus::{
    Bound,
    KeyDeserialize,
};

use crate::{
    error::ContractError,
    msg::{
        ContractRelationsQueryParams,
        ContractRelationsResponse,
        IndexValue,
        RangeQueryBound,
        RelatedAddress,
    },
    query::ReadonlyContext,
    state::storage::{
        CONTRACT_ADDR_2_ID,
        IX_REL_CONTRACT_ADDR,
    },
    util::prepare_limit_and_desc,
};

/// Query a contract's relationships (forward direction).
///
/// ## Returns
///
/// - `relations`: List of RelatedAddress containing address, name, and optional value
/// - `cursor`: Continuation token (name, address) if more results exist
///
/// ## Edge Filtering
///
/// Use start/stop NameValue bounds to filter by relationship name and/or value ranges.

pub fn query_contract_relations(
    ctx: ReadonlyContext,
    params: ContractRelationsQueryParams,
) -> Result<ContractRelationsResponse, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    let ContractRelationsQueryParams {
        contract,
        cursor,
        start,
        stop,
        limit,
        desc,
    } = params;

    let id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(contract.as_str(),)?,)?;

    let (limit, desc,) = prepare_limit_and_desc(limit, desc,);

    // Heap-allocated storage for bound byte arrays (needed for lifetime management)
    let mut from_bytes_box: Box<Vec<u8,>,> = Box::new(vec![],);

    let mut from_edge_box: Box<Vec<u8,>,> = Box::new(vec![],);

    // Build "from" bound: cursor takes precedence over start parameter
    let from_bound = match cursor {
        Some((edge, addr,),) => {

            // Cursor provided: resume from exact position (contract_id, edge, addr)
            *from_bytes_box = IndexValue::String(addr.to_string(),).to_bytes();

            *from_edge_box = edge;

            Some(Bound::Exclusive((
                (id, from_edge_box.as_slice(), from_bytes_box.as_slice(),),
                PhantomData,
            ),),)
        },
        None => {

            if let Some(start,) = start {

                // No cursor but start bound provided: use empty string as address (match all addresses)
                // The edge bytes filter by relationship name/value
                *from_bytes_box = IndexValue::String("".to_owned(),).to_bytes();

                match start {
                    RangeQueryBound::Exclusive(name_value,) => {

                        // Convert NameValue to edge bytes (name || value)
                        *from_edge_box = name_value.as_edge_bytes();

                        Some(Bound::Exclusive((
                            (id, from_edge_box.as_slice(), from_bytes_box.as_slice(),),
                            PhantomData,
                        ),),)
                    },
                    RangeQueryBound::Inclusive(name_value,) => {

                        *from_edge_box = name_value.as_edge_bytes();

                        Some(Bound::Inclusive((
                            (id, from_edge_box.as_slice(), from_bytes_box.as_slice(),),
                            PhantomData,
                        ),),)
                    },
                }
            } else {

                // No cursor and no start bound: start from beginning
                None
            }
        },
    };

    // Heap-allocated storage for stop bound byte arrays
    let mut to_bytes_box: Box<Vec<u8,>,> = Box::new(vec![],);

    let mut to_edge_box: Box<Vec<u8,>,> = Box::new(vec![],);

    // Build "to" bound from stop parameter (cursor only affects from_bound)
    let to_bound = if let Some(stop,) = stop {

        // Use empty string as address (match all addresses at this edge boundary)
        *to_bytes_box = IndexValue::String("".to_owned(),).to_bytes();

        match stop {
            RangeQueryBound::Exclusive(name_value,) => {

                *to_edge_box = name_value.as_edge_bytes();

                Some(Bound::Exclusive((
                    (id, to_edge_box.as_slice(), to_bytes_box.as_slice(),),
                    PhantomData,
                ),),)
            },
            RangeQueryBound::Inclusive(name_value,) => {

                *to_edge_box = name_value.as_edge_bytes();

                Some(Bound::Inclusive((
                    (id, to_edge_box.as_slice(), to_bytes_box.as_slice(),),
                    PhantomData,
                ),),)
            },
        }
    } else {

        None
    };

    // For descending order, flip bounds: from_bound → max, to_bound → min
    let (min_bound, max_bound, order,) = if desc {

        (to_bound, from_bound, Order::Descending,)
    } else {

        (from_bound, to_bound, Order::Ascending,)
    };

    let mut related_addrs: Vec<RelatedAddress,> = Vec::with_capacity(16,);

    // Execute range query on IX_REL_CONTRACT_ADDR: (contract_id, edge, addr) -> value
    // Deserialize edge bytes back into relationship name
    for result in IX_REL_CONTRACT_ADDR
        .range(deps.storage, min_bound, max_bound, order,)
        .take(limit,)
    {

        let ((_, name_bytes, addr_bytes,), value,) = result?;

        // Deserialize bytes back into structured data
        related_addrs.push(RelatedAddress {
            address: Addr::from_slice(addr_bytes.as_slice(),)?,
            name: String::from_vec(name_bytes,)?, // Deserialize relationship name from edge bytes
            value,                                // Value is stored separately in the index value
        },);
    }

    Ok(ContractRelationsResponse {
        // Cursor contains (name, address) from last result for continuation
        cursor: related_addrs
            .last()
            .and_then(|x| Some((x.name.to_owned(), x.address.to_owned(),),),),
        relations: related_addrs,
    },)
}
