use std::marker::PhantomData;

use cosmwasm_std::Order;
use cw_storage_plus::Bound;

use crate::{
    error::ContractError,
    msg::{PresetPaginationResponse, PresetResponse},
    query::ReadonlyContext,
    state::{models::Preset, storage::PRESETS},
};

pub fn query_preset(
    ctx: ReadonlyContext,
    name: String,
) -> Result<PresetResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;
    let preset = PRESETS.load(deps.storage, &name)?;

    Ok(PresetResponse {
        name,
        values: preset.values,
        n_uses: preset.n_uses,
        overridable: preset.overridable,
    })
}

pub fn query_paginated_presets(
    ctx: ReadonlyContext,
    cursor: Option<String>,
) -> Result<PresetPaginationResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;

    let mut boxed_name: Box<String> = Box::new(String::new());
    let min_bound = if let Some(s) = cursor {
        *boxed_name = s;
        Some(Bound::Exclusive((boxed_name.as_ref(), PhantomData)))
    } else {
        None
    };

    let mut preset_resps: Vec<PresetResponse> = Vec::with_capacity(20);
    for result in PRESETS.range(deps.storage, min_bound, None, Order::Ascending) {
        let (
            name,
            Preset {
                n_uses,
                values: value,
                overridable,
            },
        ) = result?;
        preset_resps.push(PresetResponse {
            name,
            values: value,
            n_uses,
            overridable,
        });
    }

    Ok(PresetPaginationResponse {
        cursor: preset_resps.last().and_then(|x| Some(x.name.to_owned())),
        presets: preset_resps,
    })
}
