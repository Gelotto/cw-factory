use std::marker::PhantomData;

use cosmwasm_std::{Addr, Order, Storage};
use cw_storage_plus::{Bound, Map};

use crate::{
    error::ContractError,
    msg::{ContractsByIndexResponse, ContractsInRangeQueryParams, IndexRangeBound, IndexSelector},
    query::ReadonlyContext,
    state::{
        build_index_storage_key,
        storage::{
            ContractId, IndexMap, CONTRACT_ID_2_ADDR, IX_ADMIN, IX_CODE_ID, IX_CREATED_AT, IX_CREATED_BY, IX_TAG,
            IX_UPDATED_AT,
        },
    },
};

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

pub fn query_contracts_in_range(
    ctx: ReadonlyContext,
    params: ContractsInRangeQueryParams,
) -> Result<ContractsByIndexResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;

    // Normalize limit within acceptable range
    let limit = params
        .limit
        .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT)))
        .unwrap_or(DEFAULT_LIMIT);

    // Get a vec of queried contract ID's
    let (contract_ids, cursor) = scan_index(deps.storage, &params, limit)?;

    // Look up contract addresses from ID's
    let mut contract_addrs: Vec<Addr> = Vec::with_capacity(contract_ids.len());
    for contract_id in contract_ids.iter() {
        contract_addrs.push(CONTRACT_ID_2_ADDR.load(deps.storage, *contract_id)?);
    }

    Ok(ContractsByIndexResponse {
        addresses: contract_addrs,
        cursor,
    })
}

fn scan_index(
    store: &dyn Storage,
    params: &ContractsInRangeQueryParams,
    limit: usize,
) -> Result<(Vec<ContractId>, Option<(Vec<u8>, ContractId)>), ContractError> {
    let desc = params.desc.unwrap_or_default();
    let mut custom_index_storage_key: Box<String> = Box::new(String::new());
    let mut start_bytes: Box<Vec<u8>> = Box::new(vec![]);
    let mut stop_bytes: Box<Vec<u8>> = Box::new(vec![]);

    // Get the index map to scan
    let map = match &params.index {
        IndexSelector::CreatedBy => IX_CREATED_BY,
        IndexSelector::CreatedAt => IX_CREATED_AT,
        IndexSelector::UpdatedAt => IX_UPDATED_AT,
        IndexSelector::CodeId => IX_CODE_ID,
        IndexSelector::Admin => IX_ADMIN,
        IndexSelector::Tag => IX_TAG,
        IndexSelector::Custom(index_name) => {
            *custom_index_storage_key = build_index_storage_key(&index_name);
            let map: IndexMap = Map::new(custom_index_storage_key.as_str());
            map
        },
    };

    let from_bound = match &params.cursor {
        Some((bytes, id)) => {
            *start_bytes = bytes.to_owned();
            Some(Bound::Exclusive(((start_bytes.as_slice(), *id), PhantomData)))
        },
        None => {
            let id = if desc { ContractId::MAX } else { ContractId::MIN };
            match &params.start {
                Some(bound) => match bound {
                    IndexRangeBound::Exclusive(value) => {
                        *start_bytes = value.to_bytes();
                        Some(Bound::Exclusive(((start_bytes.as_slice(), id), PhantomData)))
                    },
                    IndexRangeBound::Inclusive(value) => {
                        *start_bytes = value.to_bytes();
                        Some(Bound::Inclusive(((start_bytes.as_slice(), id), PhantomData)))
                    },
                },
                None => None,
            }
        },
    };

    let to_bound = match &params.stop {
        Some(bound) => {
            let id = if desc { ContractId::MIN } else { ContractId::MAX };
            Some(match bound {
                IndexRangeBound::Exclusive(value) => {
                    *stop_bytes = value.to_bytes();
                    Bound::Exclusive(((stop_bytes.as_slice(), id), PhantomData))
                },
                IndexRangeBound::Inclusive(value) => {
                    *stop_bytes = value.to_bytes();
                    Bound::Inclusive(((stop_bytes.as_slice(), id), PhantomData))
                },
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

    let contract_ids: Vec<ContractId> = keys.iter().map(|k| k.1).collect();
    let cursor = if keys.len() == limit {
        keys.last().and_then(|(a, b)| Some((a.to_vec(), *b)))
    } else {
        None
    };

    Ok((contract_ids, cursor))
}
