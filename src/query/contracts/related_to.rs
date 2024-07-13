use std::marker::PhantomData;

use cosmwasm_std::{Addr, Order, Storage};
use cw_storage_plus::Bound;

use crate::{
    error::ContractError,
    msg::{ContractsRelatedToParams, ContractsRelatedToResponse, IndexValue},
    query::ReadonlyContext,
    state::storage::{ContractId, CONTRACT_ID_2_ADDR, IX_REL_ADDR, IX_REL_CONTRACT_ADDR},
};

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

pub fn query_contracts_related_to(
    ctx: ReadonlyContext,
    params: ContractsRelatedToParams,
) -> Result<ContractsRelatedToResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;

    // Normalize limit within acceptable range
    let limit = params
        .limit
        .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT)))
        .unwrap_or(DEFAULT_LIMIT);

    // Get a vec of queried contract ID's
    let (ids_and_rel_values, cursor) = scan_relations(deps.storage, &params, limit)?;

    // Look up contract addresses from ID's
    let mut addresses: Vec<Addr> = Vec::with_capacity(ids_and_rel_values.len());
    let mut values: Vec<Option<String>> = Vec::with_capacity(ids_and_rel_values.len());
    for (contract_id, value) in ids_and_rel_values.iter() {
        addresses.push(CONTRACT_ID_2_ADDR.load(deps.storage, *contract_id)?);
        values.push(value.to_owned());
    }

    Ok(ContractsRelatedToResponse {
        addresses,
        values,
        cursor,
    })
}

fn scan_relations(
    store: &dyn Storage,
    params: &ContractsRelatedToParams,
    limit: usize,
) -> Result<(Vec<(ContractId, Option<String>)>, Option<ContractId>), ContractError> {
    // Convert tag to u8 slice
    let name_bytes_vec = IndexValue::String(params.name.clone()).to_bytes();
    let name_bytes = name_bytes_vec.as_slice();

    let desc = params.desc.unwrap_or_default();

    // Build the bound from which we're resuming iteration
    let from_bound = match &params.cursor {
        Some(id) => Some(Bound::Exclusive((
            (params.address.as_bytes(), name_bytes, *id),
            PhantomData,
        ))),
        None => {
            let id = if desc { ContractId::MAX } else { ContractId::MIN };
            Some(Bound::Inclusive((
                (params.address.as_bytes(), name_bytes, id),
                PhantomData,
            )))
        },
    };

    // Build the terminal bound
    let to_bound = {
        let id = if desc { ContractId::MIN } else { ContractId::MAX };
        Some(Bound::Inclusive((
            (params.address.as_bytes(), name_bytes, id),
            PhantomData,
        )))
    };

    // Prepare arguments for Map::keys
    let (min_bound, max_bound, order) = if desc {
        (to_bound, from_bound, Order::Descending)
    } else {
        (from_bound, to_bound, Order::Ascending)
    };

    // Load contract IDs and weights, ordered by weight
    // NOTE: Could be improved by just using a map.prefix()...
    let keys: Vec<_> = IX_REL_ADDR
        .keys(store, min_bound, max_bound, order)
        .map(|r| r.unwrap())
        .take(limit)
        .collect();

    let mut contract_ids_and_values: Vec<(ContractId, Option<String>)> = Vec::with_capacity(keys.len());

    for (addr, _, contract_id) in keys.iter() {
        let value = IX_REL_CONTRACT_ADDR.load(store, (*contract_id, name_bytes, &addr))?;
        contract_ids_and_values.push((*contract_id, value));
    }

    let cursor = if contract_ids_and_values.len() == limit {
        contract_ids_and_values.last().and_then(|(id, _)| Some(*id))
    } else {
        None
    };

    Ok((contract_ids_and_values, cursor))
}
