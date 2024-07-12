use cosmwasm_std::{Addr, Timestamp, Uint64};
use cw_storage_plus::{Item, Map};

use super::models::{Config, SubMsgContext};

pub type ContractId = u32;
pub type IndexMap<'a> = Map<'a, (&'a [u8], ContractId), u8>;

pub const MANAGED_BY: Item<Addr> = Item::new("managed_by");
pub const CREATED_BY: Item<Addr> = Item::new("created_by");
pub const CREATED_AT: Item<Timestamp> = Item::new("created_at");

// pub const CONFIG_MAX_SIZEOF_STRING: Item<u16> = Item::new("max_sizeof_string");
pub const CONFIG_DEFAULT_CODE_ID: Item<Uint64> = Item::new("default_code_id");
pub const CONFIG_ALLOWED_CODE_IDS: Map<u64, u8> = Map::new("allowed_code_ids");

pub const REPLY_ID_COUNTER: Item<Uint64> = Item::new("reply_id_counter");
pub const CONTRACT_ID_COUNTER: Item<u32> = Item::new("contract_id_counter");

pub const SUBMSG_CONTEXTS: Map<u64, SubMsgContext> = Map::new("submsg_contexts");

pub const CONTRACT_ID_2_ADDR: Map<ContractId, Addr> = Map::new("contract_id_2_addr");
pub const CONTRACT_ADDR_2_ID: Map<&Addr, u32> = Map::new("contract_addr_2_id");
pub const CONTRACT_COUNTER: Item<u32> = Item::new("contract_counter");

pub const IX_CODE_ID: Map<(&[u8], ContractId), u8> = Map::new("ix_code_id");
pub const IX_CREATED_AT: Map<(&[u8], ContractId), u8> = Map::new("ix_created_at");
pub const IX_UPDATED_AT: Map<(&[u8], ContractId), u8> = Map::new("ix_updated_at");
pub const IX_CREATED_BY: Map<(&[u8], ContractId), u8> = Map::new("ix_created_by");
pub const IX_ADMIN: Map<(&[u8], ContractId), u8> = Map::new("ix_admin");

pub const ID_2_CODE_ID: Map<ContractId, Vec<u8>> = Map::new("id_2_code_id");
pub const ID_2_CREATED_AT: Map<ContractId, Vec<u8>> = Map::new("id_2_created_at");
pub const ID_2_UPDATED_AT: Map<ContractId, Vec<u8>> = Map::new("id_2_updated_at");
pub const ID_2_CREATED_BY: Map<ContractId, Vec<u8>> = Map::new("id_2_created_by");
pub const ID_2_ADMIN: Map<ContractId, Vec<u8>> = Map::new("id_2_admin");
