//! Storage layout and index architecture.
//!
//! ## Index Design Philosophy
//!
//! The factory uses a "dual-map" pattern for all indices to enable both efficient
//! range queries and O(1) updates:
//!
//! 1. **Forward Index** (`IX_*`): Maps `(indexed_value, contract_id) -> marker`
//!    - Enables efficient range queries sorted by the indexed value
//!    - Composite key ensures uniqueness when same value appears on multiple contracts
//!    - Value is u8 marker (always 0) because we only need key existence
//!
//! 2. **Reverse Map** (`ID_2_*`): Maps `contract_id -> indexed_value_bytes`
//!    - Enables O(1) lookup of current index value for a contract
//!    - Required for updates: we must remove old index entry before adding new one
//!    - Stores raw bytes to avoid deserialization overhead
//!
//! ## String Key Padding
//!
//! String-based indices (tags, custom indices, relationships) use fixed-width keys
//! (128 bytes) to ensure proper lexicographic ordering in range queries. Without
//! padding, "abc" would sort incorrectly relative to "z" in byte-order comparisons.
//!
//! ## Contract ID Assignment
//!
//! Contracts receive sequential u32 IDs (separate from addresses). This enables:
//! - Compact composite index keys
//! - Efficient cursor pagination (resume from last ID)
//! - Deterministic ordering independent of address format changes

use cosmwasm_std::{
    Addr,
    Timestamp,
    Uint64,
};
use cw_storage_plus::{
    Item,
    Map,
};

use crate::msg::IndexValue;

use super::models::{
    Migration,
    MigrationError,
    Preset,
    SubMsgContext,
};

/// Internal contract ID type used for compact index keys

pub type ContractId = u32;

/// Generic index map type: (value_bytes, contract_id) -> marker

pub type IndexMap<'a,> = Map<'a, (&'a [u8], ContractId,), u8,>;

// ============================================================================
// Base Factory Metadata
// ============================================================================

/// Factory manager with permission to trigger migrations and manage presets

pub const MANAGED_BY: Item<Addr,> = Item::new("managed_by",);

/// Factory creator address (informational only)

pub const CREATED_BY: Item<Addr,> = Item::new("created_by",);

/// Factory creation timestamp

pub const CREATED_AT: Item<Timestamp,> = Item::new("created_at",);

/// Default code_id used when Create message doesn't specify one

pub const CONFIG_DEFAULT_CODE_ID: Item<Uint64,> = Item::new("default_code_id",);

/// Whitelist of code_ids that can be instantiated through this factory
/// Stored as Map for O(1) membership check (value is always 0)

pub const CONFIG_ALLOWED_CODE_IDS: Map<u64, u8,> = Map::new("allowed_code_ids",);

// ============================================================================
// ID Generators
// ============================================================================

/// Incrementing counter for reply IDs (used for both creation and migration)

pub const REPLY_ID_COUNTER: Item<Uint64,> = Item::new("reply_id_counter",);

/// Incrementing counter for contract IDs (u32 for compact index keys)

pub const CONTRACT_ID_COUNTER: Item<ContractId,> = Item::new("contract_id_counter",);

/// Temporary storage for data needed between submessage execution and reply handling
/// Maps reply_id -> context with contract metadata

pub const SUBMSG_CONTEXTS: Map<u64, SubMsgContext,> = Map::new("submsg_contexts",);

/// Total number of contracts created and managed by the factory

pub const CONTRACT_COUNTER: Item<u32,> = Item::new("contract_counter",);

// ============================================================================
// Contract Lookups (Bidirectional)
// ============================================================================

/// Primary lookup: ID -> Address

pub const CONTRACT_ID_2_ADDR: Map<ContractId, Addr,> = Map::new("contract_id_2_addr",);

/// Optional name lookup: ID -> Name

pub const CONTRACT_ID_2_NAME: Map<ContractId, String,> = Map::new("contract_id_2_name",);

/// Reverse lookup: Address -> ID (enables update auth and query resolution)

pub const CONTRACT_ADDR_2_ID: Map<&Addr, ContractId,> = Map::new("contract_addr_2_id",);

/// Reverse name lookup: Name -> ID (enables queries by human-readable name)

pub const CONTRACT_NAME_2_ID: Map<&String, ContractId,> = Map::new("contract_name_2_id",);

/// Hidden flag for contracts (future feature - currently unused)

pub const CONTRACT_ID_2_IS_HIDDEN: Map<ContractId, bool,> = Map::new("contract_id_2_is_hidden",);

/// Partition assignment for contracts (future feature - currently unused)

pub const CONTRACT_ID_2_PARTITION: Map<ContractId, u32,> = Map::new("contract_id_2_partition",);

/// Reverse lookup for custom index values: (contract_id, index_name) -> value_bytes
/// Enables O(1) retrieval of current custom index value for updates

pub const CONTRACT_CUSTOM_IX_VALUES: Map<(ContractId, &String,), Vec<u8,>,> = Map::new("custom_ix_values",);

// ============================================================================
// Built-in Indices (Forward + Reverse)
// ============================================================================

/// Index: Code ID -> Contracts
/// Enables queries like "all contracts from code_id 123"

pub const IX_CODE_ID: Map<(&[u8], ContractId,), u8,> = Map::new("ix_code_id",);

/// Index: Creation timestamp -> Contracts
/// Enables queries like "all contracts created after block X"

pub const IX_CREATED_AT: Map<(&[u8], ContractId,), u8,> = Map::new("ix_created_at",);

/// Index: Last update timestamp -> Contracts
/// Enables queries like "all contracts updated in the last 24 hours"

pub const IX_UPDATED_AT: Map<(&[u8], ContractId,), u8,> = Map::new("ix_updated_at",);

/// Index: Creator address -> Contracts
/// Enables queries like "all contracts created by address X"

pub const IX_CREATED_BY: Map<(&[u8], ContractId,), u8,> = Map::new("ix_created_by",);

/// Index: Admin address -> Contracts
/// Enables queries like "all contracts administered by address X"

pub const IX_ADMIN: Map<(&[u8], ContractId,), u8,> = Map::new("ix_admin",);

// Reverse lookups for built-in indices
// Required for updates: we must remove old index entry before adding new one

/// Reverse lookup: Contract ID -> Code ID bytes

pub const ID_2_CODE_ID: Map<ContractId, Vec<u8,>,> = Map::new("id_2_code_id",);

/// Reverse lookup: Contract ID -> Creation timestamp bytes

pub const ID_2_CREATED_AT: Map<ContractId, Vec<u8,>,> = Map::new("id_2_created_at",);

/// Reverse lookup: Contract ID -> Last update timestamp bytes

pub const ID_2_UPDATED_AT: Map<ContractId, Vec<u8,>,> = Map::new("id_2_updated_at",);

/// Reverse lookup: Contract ID -> Creator address bytes

pub const ID_2_CREATED_BY: Map<ContractId, Vec<u8,>,> = Map::new("id_2_created_by",);

/// Reverse lookup: Contract ID -> Admin address bytes

pub const ID_2_ADMIN: Map<ContractId, Vec<u8,>,> = Map::new("id_2_admin",);

// ============================================================================
// Tag System (Weighted Tags)
// ============================================================================

/// Unweighted tag index: (tag_bytes, contract_id) -> marker
/// Used for existence checks and "all contracts with tag X" queries

pub const IX_TAG: Map<(&[u8], ContractId,), u8,> = Map::new("ix_tag",);

/// Weighted tag index: (tag_bytes, weight, contract_id) -> marker
/// Enables range queries like "contracts with tag 'premium' and weight >= 50"
/// Weight is u16 to allow fine-grained prioritization (0-65535)

pub const IX_WEIGHTED_TAG: Map<(&[u8], u16, ContractId,), u8,> = Map::new("ix_weighted_tag",);

/// Weight lookup: (contract_id, tag_bytes) -> weight
/// Enables O(1) retrieval of current weight for updates and queries

pub const CONTRACT_TAG_WEIGHTS: Map<(ContractId, &[u8],), u16,> = Map::new("contract_tag_weights",);

// ============================================================================
// Relationship Graph
// ============================================================================

/// Forward relationship index: (from_contract_id, edge_bytes, to_addr_bytes) -> optional_value
/// Edge encoding: name_bytes || value_bytes (value is optional)
/// Enables queries like "get all relationships from contract X to address Y with name 'depends_on'"

pub const IX_REL_CONTRACT_ADDR: Map<(ContractId, &[u8], &[u8],), Option<IndexValue,>,> =
    Map::new("ix_rel_contract_addr",);

/// Reverse relationship index: (to_addr_bytes, edge_bytes, from_contract_id) -> marker
/// Enables queries like "all contracts that depend on address Y"

pub const IX_REL_ADDR: Map<(&[u8], &[u8], ContractId,), u8,> = Map::new("ix_rel_addr",);

// ============================================================================
// Preset Templates
// ============================================================================

/// Preset templates keyed by user-defined name

pub const PRESETS: Map<&String, Preset,> = Map::new("presets",);

// ============================================================================
// Migration Session State
// ============================================================================

/// Active migration sessions keyed by user-defined name

pub const MIGRATIONS: Map<&String, Migration,> = Map::new("migrations",);

/// Maps reply_id -> (session_name, contract_id) for migration submessages
/// Checked in reply handler to distinguish migration replies from creation replies

pub const MIGRATION_REPLY_ID_2_STATE: Map<u64, (String, ContractId,),> = Map::new("migration_reply_id_2_name",);

/// Failed migrations: (session_name, contract_id) -> error details
/// Enables retry logic and error tracking for batch migrations

pub const MIGRATION_ERRORS: Map<(&String, ContractId,), MigrationError,> = Map::new("migration_errors",);
