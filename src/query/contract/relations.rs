use std::marker::PhantomData;

use cosmwasm_std::{Addr, Order};
use cw_storage_plus::{Bound, KeyDeserialize};

use crate::{
    error::ContractError,
    msg::{ContractRelationsQueryParams, ContractRelationsResponse, IndexValue, RangeQueryBound, RelatedAddress},
    query::ReadonlyContext,
    state::storage::{CONTRACT_ADDR_2_ID, IX_REL_CONTRACT_ADDR},
    util::prepare_limit_and_desc,
};

pub fn query_contract_relations(
    ctx: ReadonlyContext,
    params: ContractRelationsQueryParams,
) -> Result<ContractRelationsResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;
    let ContractRelationsQueryParams {
        address: contract,
        cursor,
        start,
        stop,
        limit,
        desc,
    } = params;

    let id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(contract.as_str())?)?;

    let (limit, desc) = prepare_limit_and_desc(limit, desc);

    // Set the starting point for iterating over tags
    let mut from_bytes_box: Box<Vec<u8>> = Box::new(vec![]);
    let mut from_addr_box: Box<Addr> = Box::new(Addr::unchecked(""));

    let from_bound = match cursor {
        // Continue iterating after cursor
        Some((name, addr)) => {
            *from_bytes_box = IndexValue::String(name).to_bytes();
            *from_addr_box = addr;
            let bytes = from_bytes_box.as_slice();
            Some(Bound::Exclusive(((id, bytes, from_addr_box.as_bytes()), PhantomData)))
        },
        // Start iterating from the beginning or from the start given.
        None => {
            if let Some(start) = start {
                match start {
                    RangeQueryBound::Exclusive(name) => {
                        *from_bytes_box = IndexValue::String(name.to_owned()).to_bytes();
                        let bytes = from_bytes_box.as_slice();
                        Some(Bound::Exclusive(((id, bytes, from_addr_box.as_bytes()), PhantomData)))
                    },
                    RangeQueryBound::Inclusive(name) => {
                        *from_bytes_box = IndexValue::String(name.to_owned()).to_bytes();
                        let bytes = from_bytes_box.as_slice();
                        Some(Bound::Exclusive(((id, bytes, from_addr_box.as_bytes()), PhantomData)))
                    },
                }
            } else {
                None
            }
        },
    };

    // Set the bound where iteration should stop
    let mut to_bytes_box: Box<Vec<u8>> = Box::new(vec![]);
    let mut to_addr_box: Box<Vec<u8>> = Box::new(vec![]);
    let to_bound = if let Some(stop) = stop {
        *to_addr_box = {
            let mut liminal_addr_bytes = contract.as_bytes().to_owned();
            if desc {
                liminal_addr_bytes.fill(u8::MIN)
            } else {
                liminal_addr_bytes.fill(u8::MAX)
            }
            liminal_addr_bytes
        };
        match stop {
            RangeQueryBound::Exclusive(name) => {
                *to_bytes_box = IndexValue::String(name.to_owned()).to_bytes();
                let bytes = to_bytes_box.as_slice();
                Some(Bound::Exclusive(((id, bytes, to_addr_box.as_slice()), PhantomData)))
            },
            RangeQueryBound::Inclusive(name) => {
                *to_bytes_box = IndexValue::String(name.to_owned()).to_bytes();
                let bytes = to_bytes_box.as_slice();
                Some(Bound::Inclusive(((id, bytes, to_addr_box.as_slice()), PhantomData)))
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

    let mut related_addrs: Vec<RelatedAddress> = Vec::with_capacity(16);

    // Build up returned weighed tags vec
    for result in IX_REL_CONTRACT_ADDR
        .range(deps.storage, min_bound, max_bound, order)
        .take(limit)
    {
        let ((_, name_bytes, addr_bytes), value) = result?;
        related_addrs.push(RelatedAddress {
            address: Addr::from_slice(addr_bytes.as_slice())?,
            name: String::from_vec(name_bytes)?,
            value,
        });
    }

    Ok(ContractRelationsResponse {
        cursor: related_addrs
            .last()
            .and_then(|x| Some((x.name.to_owned(), x.address.to_owned()))),
        relations: related_addrs,
    })
}
