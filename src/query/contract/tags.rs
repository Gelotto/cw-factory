use std::marker::PhantomData;

use cosmwasm_std::Order;
use cw_storage_plus::{Bound, KeyDeserialize};

use crate::{
    error::ContractError,
    msg::{ContractTagsQueryParams, ContractTagsResponse, IndexValue, RangeQueryBound, WeightedTag},
    query::ReadonlyContext,
    state::storage::{CONTRACT_ADDR_2_ID, CONTRACT_TAG_WEIGHTS},
    util::prepare_limit_and_desc,
};

pub fn query_contract_tags(
    ctx: ReadonlyContext,
    params: ContractTagsQueryParams,
) -> Result<ContractTagsResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;
    let ContractTagsQueryParams {
        contract,
        cursor,
        start,
        stop,
        limit,
        desc,
    } = params;

    let contract_id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(contract.as_str())?)?;

    let (limit, desc) = prepare_limit_and_desc(limit, desc);

    // Set the starting point for iterating over tags
    let mut from_bytes_box: Box<Vec<u8>> = Box::new(vec![]);
    let from_bound = match cursor {
        // Continue iterating starting from the map item after cursor tag.
        Some(cursor_tag) => {
            *from_bytes_box = IndexValue::String(cursor_tag).to_bytes();
            let bytes = from_bytes_box.as_slice();
            Some(Bound::Exclusive((bytes, PhantomData)))
        },
        // Start iterating from the beginning or from given "start" tag.
        None => {
            if let Some(start) = start {
                match start {
                    RangeQueryBound::Exclusive(tag) => {
                        *from_bytes_box = IndexValue::String(tag.to_owned()).to_bytes();
                        let bytes = from_bytes_box.as_slice();
                        Some(Bound::Exclusive((bytes, PhantomData)))
                    },
                    RangeQueryBound::Inclusive(tag) => {
                        *from_bytes_box = IndexValue::String(tag.to_owned()).to_bytes();
                        let bytes = from_bytes_box.as_slice();
                        Some(Bound::Inclusive((bytes, PhantomData)))
                    },
                }
            } else {
                None
            }
        },
    };

    // Set the bound where iteration should stop
    let mut to_bytes_box: Box<Vec<u8>> = Box::new(vec![]);
    let to_bound = if let Some(stop) = stop {
        match stop {
            RangeQueryBound::Exclusive(tag) => {
                *to_bytes_box = IndexValue::String(tag.to_owned()).to_bytes();
                let bytes = to_bytes_box.as_slice();
                Some(Bound::Exclusive((bytes, PhantomData)))
            },
            RangeQueryBound::Inclusive(tag) => {
                *to_bytes_box = IndexValue::String(tag.to_owned()).to_bytes();
                let bytes = to_bytes_box.as_slice();
                Some(Bound::Inclusive((bytes, PhantomData)))
            },
        }
    } else {
        None
    };

    // Prepare args for Map::range
    let (min_bound, max_bound, order) = if desc {
        (to_bound, from_bound, Order::Descending)
    } else {
        (from_bound, to_bound, Order::Ascending)
    };

    let mut weighted_tags: Vec<WeightedTag> = Vec::with_capacity(4);

    // Build up returned weighed tags vec
    for result in CONTRACT_TAG_WEIGHTS
        .prefix(contract_id)
        .range(deps.storage, min_bound, max_bound, order)
        .take(limit)
    {
        let (tag_bytes, weight) = result?;
        weighted_tags.push(WeightedTag {
            tag: String::from_vec(tag_bytes)?,
            weight,
        });
    }

    Ok(ContractTagsResponse {
        cursor: weighted_tags.last().and_then(|t| Some(t.tag.to_owned())),
        tags: weighted_tags,
    })
}
