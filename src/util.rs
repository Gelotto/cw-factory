//! Utility functions for preset merging, pagination, padding, and authorization.
//!
//! ## Key Functions
//!
//! - `apply_preset`: Merges preset templates with user instantiate messages
//! - `pad_vec` / `unpad_vec`: String key padding for lexicographic ordering
//! - `ensure_is_manager`: Authorization check for manager-only operations
//! - `prepare_limit_and_desc`: Normalizes pagination parameters

use base64::{
    engine::general_purpose::URL_SAFE as BASE64,
    Engine as _,
};
use cosmwasm_std::{
    ensure_eq,
    Addr,
    Binary,
    StdError,
    StdResult,
    Storage,
};
use serde_json::{
    self,
    Map,
    Value,
};

use crate::{
    error::ContractError,
    state::storage::{
        MANAGED_BY,
        PRESETS,
    },
};

/// Default pagination limit for queries

const DEFAULT_LIMIT: usize = 100;

/// Maximum pagination limit to prevent excessive gas usage

const MAX_LIMIT: usize = 500;

/// Normalize and validate pagination parameters.
///
/// ## Parameters
///
/// - `limit`: Optional page size (clamped to 1-500, default 100)
/// - `desc`: Optional descending order flag (default false)
///
/// ## Returns
///
/// Tuple of (normalized_limit, descending_flag)

pub fn prepare_limit_and_desc(
    limit: Option<u16,>,
    desc: Option<bool,>,
) -> (usize, bool,) {

    (
        limit
            .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT,),),)
            .unwrap_or(DEFAULT_LIMIT,),
        desc.unwrap_or_default(),
    )
}

/// Merge a preset template with user-provided instantiate message.
///
/// ## Merge Strategy
///
/// - **Preset overridable=true**: Preset values are defaults, user values override
/// - **Preset overridable=false**: Preset values are locked, user values are ignored
///
/// ## Process
///
/// 1. Load preset if name provided
/// 2. Merge JSON objects based on overridable flag
/// 3. Serialize to Base64-encoded JSON Binary for WasmMsg::Instantiate

pub fn apply_preset(
    store: &dyn Storage,
    client_instantiate_msg: Map<String, Value,>,
    maybe_preset_name: Option<String,>,
) -> StdResult<Binary,> {

    // Merge preset object into custom instantiate_msg (or vice versa depending on overridable flag)
    let msg = if let Some(preset_name,) = &maybe_preset_name {

        let preset = PRESETS.load(store, preset_name,)?;

        // Determine merge direction based on overridable flag
        let (mut dst, src,) = if preset.overridable {

            // Overridable: preset is base, user values override
            (preset.values, client_instantiate_msg,)
        } else {

            // Not overridable: user is base, preset values override (locked preset)
            (client_instantiate_msg, preset.values,)
        };

        // Merge source into destination (dst values get overwritten by src)
        for (k, v,) in src.iter() {

            dst.insert(k.to_owned(), v.to_owned(),);
        }

        dst
    } else {

        // No preset: use user message as-is
        client_instantiate_msg
    };

    // Encode as Base64 JSON Binary for WasmMsg::Instantiate
    let json_str =
        serde_json::to_string(&msg,).map_err(|e| ContractError::Std(StdError::generic_err(e.to_string(),),),)?;

    let b64_encoded = BASE64.encode(json_str,);

    Binary::from_base64(&b64_encoded,)
}

/// Verify that the given address is the factory manager.
///
/// Returns `NotAuthorized` error if address does not match MANAGED_BY.

pub fn ensure_is_manager(
    store: &dyn Storage,
    addr: &Addr,
) -> Result<(), ContractError,> {

    ensure_eq!(
        addr,
        MANAGED_BY.load(store)?,
        ContractError::NotAuthorized {
            reason: "only manager can set presets".to_owned()
        }
    );

    Ok((),)
}

/// Remove trailing zero bytes from a vector.
///
/// ## Purpose
///
/// Strips padding added by `pad_vec` to reduce cursor payload size.
/// Used when returning cursors to clients.

pub fn unpad_vec(bytes: Vec<u8,>,) -> Vec<u8,> {

    let len = bytes.len();

    let mut i = len - 1;

    let mut bytes = bytes;

    // Remove trailing zeros
    while i != 0 && bytes[i] == 0 {

        bytes.pop();

        i -= 1;
    }

    bytes
}

/// Pad a vector with trailing zeros to a target length.
///
/// ## Purpose
///
/// Ensures fixed-width keys for lexicographic ordering in storage.
/// String indices use 128-byte padding so "abc" sorts correctly relative to "z".
///
/// ## Parameters
///
/// - `vec`: Input bytes to pad
/// - `target_length`: Desired length (padding added if vec is shorter)

pub fn pad_vec(
    vec: Vec<u8,>,
    target_length: usize,
) -> Vec<u8,> {

    let n = target_length.saturating_sub(vec.len(),);

    if n > 0 {

        let mut padded_vec = vec;

        padded_vec.reserve(n,);

        // Add zero bytes for padding
        for _ in 0..n {

            padded_vec.push(0,)
        }

        padded_vec
    } else {

        // Already at or above target length: return as-is
        vec
    }
}
