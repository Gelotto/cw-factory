use std::marker::PhantomData;

use cosmwasm_std::{Addr, Order, Storage};
use cw_storage_plus::Bound;

use crate::{
    error::ContractError,
    msg::{ContractsByTagParams, ContractsByTagResponse, IndexValue, TagWeightRangeBound},
    query::ReadonlyContext,
    state::storage::{ContractId, CONTRACT_ID_2_ADDR, IX_WEIGHTED_TAG},
};

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

pub fn query_contracts_by_tag(
    ctx: ReadonlyContext,
    params: ContractsByTagParams,
) -> Result<ContractsByTagResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;

    // Normalize limit within acceptable range
    let limit = params
        .limit
        .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT)))
        .unwrap_or(DEFAULT_LIMIT);

    // Get a vec of queried contract ID's
    let (contract_id_weights, cursor) = scan_tag(deps.storage, &params, limit)?;

    // Look up contract addresses from ID's
    let mut addresses: Vec<Addr> = Vec::with_capacity(contract_id_weights.len());
    let mut weights: Vec<u16> = Vec::with_capacity(contract_id_weights.len());
    for (contract_id, weight) in contract_id_weights.iter() {
        addresses.push(CONTRACT_ID_2_ADDR.load(deps.storage, *contract_id)?);
        weights.push(*weight);
    }

    Ok(ContractsByTagResponse {
        addresses,
        weights,
        cursor,
    })
}

fn scan_tag(
    store: &dyn Storage,
    params: &ContractsByTagParams,
    limit: usize,
) -> Result<(Vec<(ContractId, u16)>, Option<(Vec<u8>, u16, ContractId)>), ContractError> {
    let map = IX_WEIGHTED_TAG;

    // Convert tag to u8 slice
    let bytes_vec = IndexValue::String(params.tag.clone()).to_bytes();
    let bytes = bytes_vec.as_slice();

    let desc = params.desc.unwrap_or_default();

    let from_bound = match &params.cursor {
        Some((cursor_bytes, w, id)) => Some(Bound::Exclusive(((cursor_bytes.as_slice(), *w, *id), PhantomData))),
        None => {
            let id = if desc { ContractId::MAX } else { ContractId::MIN };
            match &params.min_weight {
                Some(bound) => match bound {
                    TagWeightRangeBound::Exclusive(weight) => {
                        Some(Bound::Exclusive(((bytes, *weight, id), PhantomData)))
                    },
                    TagWeightRangeBound::Inclusive(weight) => {
                        Some(Bound::Inclusive(((bytes, *weight, id), PhantomData)))
                    },
                },
                None => None,
            }
        },
    };

    let to_bound = match &params.max_weight {
        Some(bound) => {
            let id = if desc { ContractId::MIN } else { ContractId::MAX };
            Some(match bound {
                &TagWeightRangeBound::Exclusive(w) => Bound::Exclusive(((bytes, w, id), PhantomData)),
                &TagWeightRangeBound::Inclusive(w) => Bound::Inclusive(((bytes, w, id), PhantomData)),
            })
        },
        None => None,
    };

    let (min_bound, max_bound, order) = if desc {
        (to_bound, from_bound, Order::Descending)
    } else {
        (from_bound, to_bound, Order::Ascending)
    };

    let keys: Vec<_> = map
        .keys(store, min_bound, max_bound, order)
        .take(limit)
        .map(|r| r.unwrap())
        .collect();

    let contract_ids: Vec<(ContractId, u16)> = keys.iter().map(|k| (k.2, k.1)).collect();
    let cursor = if keys.len() == limit {
        keys.last().and_then(|(a, b, c)| Some((a.to_vec(), *b, *c)))
    } else {
        None
    };

    Ok((contract_ids, cursor))
}
