use base64::{engine::general_purpose::URL_SAFE as BASE64, Engine as _};
use cosmwasm_std::{ensure_eq, Addr, Binary, StdError, StdResult, Storage};
use serde_json::{self, Map, Value};

use crate::{
    error::ContractError,
    state::storage::{MANAGED_BY, PRESETS},
};

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

pub fn prepare_limit_and_desc(
    limit: Option<u16>,
    desc: Option<bool>,
) -> (usize, bool) {
    (
        limit
            .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT)))
            .unwrap_or(DEFAULT_LIMIT),
        desc.unwrap_or_default(),
    )
}

pub fn apply_preset(
    store: &dyn Storage,
    client_instantiate_msg: Map<String, Value>,
    maybe_preset_name: Option<String>,
) -> StdResult<Binary> {
    // merge preset object into custom instantiate_msg -- or vice versa
    let msg = if let Some(preset_name) = &maybe_preset_name {
        let preset = PRESETS.load(store, preset_name)?;
        let (mut dst, src) = if preset.overridable {
            (preset.values, client_instantiate_msg)
        } else {
            (client_instantiate_msg, preset.values)
        };
        for (k, v) in src.iter() {
            dst.insert(k.to_owned(), v.to_owned());
        }
        dst
    } else {
        client_instantiate_msg
    };

    // Encode as b64 json object and convert to binary
    let json_str = serde_json::to_string(&msg).map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))?;
    let b64_encoded = BASE64.encode(json_str);

    Binary::from_base64(&b64_encoded)
}

pub fn ensure_is_manager(
    store: &dyn Storage,
    addr: &Addr,
) -> Result<(), ContractError> {
    ensure_eq!(
        addr,
        MANAGED_BY.load(store)?,
        ContractError::NotAuthorized {
            reason: "only manager can set presets".to_owned()
        }
    );
    Ok(())
}

pub fn unpad_vec(bytes: Vec<u8>) -> Vec<u8> {
    let len = bytes.len();
    let mut i = len - 1;
    let mut bytes = bytes;
    while i != 0 && bytes[i] == 0 {
        bytes.pop();
        i -= 1;
    }
    bytes
}

pub fn pad_vec(
    vec: Vec<u8>,
    target_length: usize,
) -> Vec<u8> {
    let n = target_length.saturating_sub(vec.len());
    if n > 0 {
        let mut padded_vec = vec;
        padded_vec.reserve(n);
        for _ in 0..n {
            padded_vec.push(0)
        }
        padded_vec
    } else {
        vec
    }
}
