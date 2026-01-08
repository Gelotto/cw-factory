# CosmWasm Factory

> A production-ready smart contract factory with built-in indexing, tagging, and graph relationships for CosmWasm 1.0+

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![CosmWasm](https://img.shields.io/badge/CosmWasm-1.5.2-blue)](https://github.com/CosmWasm/cosmwasm)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

## Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Usage Examples](#usage-examples)
- [API Reference](#api-reference)
- [Advanced Features](#advanced-features)
- [Building & Testing](#building--testing)
- [Deployment](#deployment)
- [Performance & Security](#performance--security)
- [FAQ](#faq)
- [License](#license)

## Overview

CosmWasm Factory is a **database-on-blockchain** primitive that transforms contract management into a queryable, indexed data structure. Unlike traditional factory patterns that simply instantiate contracts, this factory provides SQL-like querying, sophisticated indexing, weighted tagging, and graph-based relationships between contracts—all on-chain.

**Think of it as:** PostgreSQL meets Factory Pattern for CosmWasm smart contracts.

### Why Use This Factory?

- **Zero Off-Chain Dependencies**: Build complete backend stacks entirely on-chain without indexers or databases
- **SQL-Like Queries**: Range queries, filtering, sorting, and pagination out of the box
- **Custom Indexing**: Add arbitrary typed indexes to contracts for efficient lookups (13+ supported types)
- **Graph Relationships**: Create named relationships between contracts with optional typed values
- **Batch Migrations**: Migrate hundreds of contracts with error handling and retry logic
- **Production Ready**: Well-tested patterns, comprehensive error handling, gas-optimized

### Factory Pattern Flow

```
┌─────────────────┐
│   Your dApp     │
└────────┬────────┘
         │ create(...)
         ↓
┌─────────────────────────────────────────┐
│         CosmWasm Factory                │
│  ┌────────────┐  ┌─────────────────┐   │
│  │ Contracts  │  │    Indexes      │   │
│  │  Registry  │←→│  • Built-in     │   │
│  └────────────┘  │  • Custom       │   │
│  ┌────────────┐  │  • Tags         │   │
│  │  Relations │  │  • Timestamps   │   │
│  │   Graph    │  └─────────────────┘   │
│  └────────────┘                         │
└─────────────────────────────────────────┘
         │ instantiate + index
         ↓
┌─────────────────┐
│ Child Contracts │
└─────────────────┘
```

## Key Features

### Smart Indexing

- **Built-in Indexes**: `created_by`, `created_at`, `updated_at`, `code_id`, `admin`
- **Custom Indexes**: Add arbitrary typed indexes with 13+ supported types (`Uint*`, `Int*`, `String`, `Bytes`, `Binary`, `Bool`)
- **Efficient Pagination**: Cursor-based pagination with configurable limits (up to 500)
- **Range Queries**: Filter by inclusive/exclusive bounds on any indexed field

### Weighted Tagging System

- Assign multiple tags to contracts with numeric weights (0-65535)
- Query contracts by tag with weight range filtering
- Tag-based filtering with AND/OR/XOR boolean operations
- Perfect for categorization, feature flags, or priority systems

### Graph Relationships

- Create bidirectional relationships: `contractA ←[name: value]→ addressB`
- Named relationships with optional typed values
- Query all contracts related to a specific address
- Relationship range queries by name and value
- Build complex contract dependency graphs

### Preset System

- Define reusable instantiation templates
- Override protection (lockable/flexible presets)
- Track usage statistics
- Simplify contract creation for common patterns

### Batch Migrations

- Migrate hundreds of contracts in batches
- Session-based migration with pause/resume
- Error handling strategies: Abort or Retry
- Track success/failure rates
- Filter by source code_id

### Rich Querying

- Query contracts in ranges by any index
- Filter by tags with weight constraints
- Find all contracts related to an address
- Boolean tag tests (AND/OR/XOR)
- Metadata retrieval with timestamps

## Architecture

### Storage Design

The factory uses a **compound index pattern** for O(1) lookups and efficient range scans:

```rust
// Primary Storage
CONTRACT_ID_2_ADDR:    Map<u32, Addr>        // ID → Address
CONTRACT_ADDR_2_ID:    Map<Addr, u32>        // Address → ID
CONTRACT_NAME_2_ID:    Map<String, u32>      // Name → ID (optional)

// Built-in Indexes (bytes, u32) → u8
IX_CREATED_BY:         Map<(&[u8], u32), u8>
IX_CREATED_AT:         Map<(&[u8], u32), u8>
IX_CODE_ID:            Map<(&[u8], u32), u8>

// Custom Indexes (dynamic)
IX_CUSTOM_{name}:      Map<(&[u8], u32), u8>

// Weighted Tags
IX_WEIGHTED_TAG:       Map<(&[u8], u16, u32), u8>  // (tag, weight, id)
IX_TAG:                Map<(&[u8], u32), u8>       // (tag, id)

// Graph Relations
IX_REL_CONTRACT_ADDR:  Map<(u32, &[u8], &[u8]), Option<IndexValue>>
IX_REL_ADDR:           Map<(&[u8], &[u8], u32), u8>
```

### Index Update Flow

```
1. Contract created via factory
2. Factory instantiates child contract (SubMsg)
3. Reply handler extracts contract address
4. Built-in indexes populated automatically
5. Child contract calls factory.update() via SubMsg
6. Custom indexes, tags, relations updated
```

### Permission Model

- **Creator**: Can instantiate contracts through factory
- **Manager**: Full control (migrations, presets, manual updates)
- **Child Contract**: Can update own indexes via SubMsg callback
- **Factory as Admin**: Factory is admin of created contracts (enables batch migrations)

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cw-factory = { git = "https://github.com/your-org/cw-factory", tag = "v1.0.0" }
```

### 1. Instantiate the Factory

```rust
use cw_factory::msg::{InstantiateMsg, Config};

let msg = InstantiateMsg {
    config: Config {
        managed_by: Addr::unchecked("manager_address"),
        default_code_id: Some(Uint64::from(123u64)),
        allowed_code_ids: vec![Uint64::from(123u64), Uint64::from(456u64)],
    },
};
```

### 2. Create Your First Contract

```rust
use cw_factory::msg::{ExecuteMsg, CreateMsg};

let create_msg = CreateMsg {
    code_id: Some(Uint64::from(123u64)),
    label: "my-contract-1".to_string(),
    name: Some("my-contract".to_string()),
    admin: None, // Factory becomes admin
    tags: Some(vec!["production".to_string(), "verified".to_string()]),
    preset: None,
    instantiate_msg: json!({
        "name": "My Contract",
        "symbol": "MC"
    }).as_object().unwrap().clone(),
};

let msg = ExecuteMsg::Create(create_msg);
```

### 3. Query Contracts by Tag

```rust
use cw_factory::msg::{QueryMsg, ContractSetQueryMsg, ContractsByTagQueryParams};

let query = QueryMsg::Contracts(
    ContractSetQueryMsg::WithTag(ContractsByTagQueryParams {
        tag: "production".to_string(),
        min_weight: None,
        max_weight: None,
        limit: Some(50),
        desc: Some(false),
        cursor: None,
    })
);
```

## Core Concepts

### Contract Identifiers

Contracts can be referenced in three ways:

```rust
pub enum ContractSelector {
    Address(Addr),           // By blockchain address
    Id(ContractId),          // By factory internal ID (u32)
    Name(String),            // By optional human-readable name
}
```

### Index Value Types

The factory supports 13 typed index values with proper byte serialization:

```rust
pub enum IndexValue {
    String(String),          // Max 128 bytes, padded
    Bytes(Vec<u8>),
    Binary(Binary),
    Uint128(Uint128), Uint64(Uint64), Uint32(u32), Uint16(u16), Uint8(u8),
    Int128(Int128), Int64(Int64), Int32(i32), Int16(i16), Int8(i8),
    Bool(bool),
}
```

**Why typed values?** Proper byte ordering ensures correct range queries and sorting.

### Update Operations

Contracts can update their own metadata via SubMsg callbacks:

```rust
pub struct UpdateMsg {
    pub contract: Option<ContractSelector>,  // None = msg.sender
    pub indices: Option<Vec<IndexUpdate>>,
    pub tags: Option<Vec<TagUpdate>>,
    pub relations: Option<Vec<RelationUpdate>>,
}
```

**Example: Child contract updating its index**

```rust
// In your child contract's execute handler
let update_msg = WasmMsg::Execute {
    contract_addr: factory_addr.to_string(),
    msg: to_binary(&FactoryExecuteMsg::Update(UpdateMsg {
        contract: None, // Self
        indices: Some(vec![
            IndexUpdate {
                name: "total_supply".to_string(),
                value: IndexValue::Uint128(Uint128::from(1000000u128)),
            }
        ]),
        tags: Some(vec![
            TagUpdate {
                op: UpdateOperation::Set,
                tag: "fully-minted".to_string(),
                weight: Some(100),
            }
        ]),
        relations: None,
    }))?,
    funds: vec![],
};

Ok(Response::new().add_submessage(SubMsg::new(update_msg)))
```

### Boolean Tag Tests

Query contracts that match tag criteria with logical operations:

```rust
pub enum BooleanTest {
    And,  // All tags must be present
    Or,   // Any tag must be present
    Xor,  // Exactly one tag must be present
}

// Example: Find contracts with BOTH "verified" AND "production" tags
ContractHasTagsQueryParams {
    address: contract_addr,
    test: BooleanTest::And,
    tags: vec![
        TagSelector { tag: "verified".to_string(), min_weight: None, max_weight: None },
        TagSelector { tag: "production".to_string(), min_weight: Some(50), max_weight: None },
    ],
}
```

## Usage Examples

### Example 1: NFT Collection Factory

Create and index multiple NFT collections:

```rust
// Create NFT collection
let create_msg = CreateMsg {
    code_id: Some(Uint64::from(789u64)),
    label: "Cool Cats NFT".to_string(),
    name: Some("cool-cats".to_string()),
    admin: Some(creator.clone()),
    tags: Some(vec!["nft".to_string(), "collection".to_string()]),
    instantiate_msg: json!({
        "name": "Cool Cats",
        "symbol": "CATS",
        "minter": creator.to_string(),
    }).as_object().unwrap().clone(),
    preset: None,
};

// Later: Find all NFT collections created by a user
let query = QueryMsg::Contracts(
    ContractSetQueryMsg::InRange(ContractsInRangeQueryParams {
        index: IndexSelector::CreatedBy,
        start: Some(IndexRangeBound::Inclusive(
            IndexValue::String(creator.to_string())
        )),
        stop: Some(IndexRangeBound::Inclusive(
            IndexValue::String(creator.to_string())
        )),
        limit: Some(100),
        desc: Some(true),
        cursor: None,
    })
);
```

### Example 2: DAO Treasury Management

Track relationships between DAOs and their treasury contracts:

```rust
// Update DAO to link to its treasury
let update_msg = UpdateMsg {
    contract: Some(ContractSelector::Name("my-dao".to_string())),
    indices: None,
    tags: None,
    relations: Some(vec![
        RelationUpdate {
            op: UpdateOperation::Set,
            name: "treasury".to_string(),
            value: Some(IndexValue::String("primary".to_string())),
            address: treasury_addr.clone(),
        }
    ]),
};

// Query all DAOs that have a specific address as treasury
let query = QueryMsg::Contracts(
    ContractSetQueryMsg::RelatedTo(ContractsRelatedToParams {
        address: treasury_addr,
        start: Some(RangeQueryBound::Inclusive(NameValue {
            name: "treasury".to_string(),
            value: None,
        })),
        stop: Some(RangeQueryBound::Inclusive(NameValue {
            name: "treasury".to_string(),
            value: None,
        })),
        limit: Some(50),
        desc: None,
        cursor: None,
    })
);
```

### Example 3: Using Presets for Standardized Contracts

```rust
// Manager sets up a preset for token contracts
let preset_msg = SetPresetMsg {
    name: "standard-token".to_string(),
    overridable: true,
    values: json!({
        "decimals": 6,
        "marketing": {
            "project": "MyProject",
            "logo": { "url": "https://example.com/logo.png" }
        }
    }).as_object().unwrap().clone(),
};

// Users create tokens with preset + custom values
let create_msg = CreateMsg {
    code_id: None, // Use default
    label: "My Token".to_string(),
    name: Some("my-token".to_string()),
    admin: None,
    tags: Some(vec!["token".to_string()]),
    preset: Some("standard-token".to_string()),
    instantiate_msg: json!({
        "name": "My Token",
        "symbol": "MTK",
        "initial_balances": [...]
        // decimals and marketing come from preset
    }).as_object().unwrap().clone(),
};
```

### Example 4: Batch Migration

Migrate all contracts from code_id 1 to code_id 2:

```rust
// 1. Begin migration session
let begin_msg = ExecuteMsg::Migrations(
    MigrationsExecuteMsg::Session(
        MigrationSessionMsg::Begin(MigrationParams {
            name: "v1-to-v2".to_string(),
            from_code_id: Some(Uint64::from(1u64)),
            to_code_id: Uint64::from(2u64),
            migrate_msg: Some(to_binary(&MyMigrateMsg {})?),
            batch_size: Some(50), // Process 50 at a time
            error_strategy: MigrationErrorStrategy::Retry,
        })
    )
);

// 2. Step through migration (call multiple times until complete)
let step_msg = ExecuteMsg::Migrations(
    MigrationsExecuteMsg::Session(
        MigrationSessionMsg::Step {
            name: "v1-to-v2".to_string()
        }
    )
);

// 3. Check status
let status_query = QueryMsg::Migrations(
    MigrationsQueryMsg::Session("v1-to-v2".to_string())
);

// 4. Retry failed migrations
let retry_msg = ExecuteMsg::Migrations(
    MigrationsExecuteMsg::Session(
        MigrationSessionMsg::Retry {
            name: "v1-to-v2".to_string(),
            params: None, // Or provide updated params
        }
    )
);
```

### Example 5: Weighted Tag Priority System

```rust
// Set tags with weights for priority
let update_msg = UpdateMsg {
    contract: None,
    indices: None,
    tags: Some(vec![
        TagUpdate {
            op: UpdateOperation::Set,
            tag: "priority".to_string(),
            weight: Some(100), // High priority
        }
    ]),
    relations: None,
};

// Query high-priority contracts (weight >= 50)
let query = QueryMsg::Contracts(
    ContractSetQueryMsg::WithTag(ContractsByTagQueryParams {
        tag: "priority".to_string(),
        min_weight: Some(TagWeightRangeBound::Inclusive(50)),
        max_weight: None,
        limit: Some(100),
        desc: Some(true), // Highest priority first
        cursor: None,
    })
);
```

## API Reference

### Execute Messages

#### `Create`

Instantiate a new contract through the factory.

```rust
ExecuteMsg::Create(CreateMsg {
    code_id: Option<Uint64>,           // If None, uses default_code_id
    label: String,                     // Required: contract label
    name: Option<String>,              // Optional: unique name for lookup
    admin: Option<Addr>,               // If None, factory is admin
    tags: Option<Vec<String>>,         // Initial tags (weight = 0)
    preset: Option<String>,            // Apply preset template
    instantiate_msg: Map<String, Value>, // JSON instantiate message
})
```

**Permissions:** Any address
**Returns:** `factory-create` event with `contract_address`, `code_id`, `admin`

#### `Update`

Update contract indexes, tags, or relationships.

```rust
ExecuteMsg::Update(UpdateMsg {
    contract: Option<ContractSelector>, // None = msg.sender
    indices: Option<Vec<IndexUpdate>>,
    tags: Option<Vec<TagUpdate>>,
    relations: Option<Vec<RelationUpdate>>,
})
```

**Permissions:**
- Contract itself (when `contract` is None)
- Factory manager (when `contract` is specified)

**Sub-types:**

```rust
IndexUpdate {
    name: String,          // Index name
    value: IndexValue,     // Typed value
}

TagUpdate {
    op: UpdateOperation,   // Set | Remove
    tag: String,
    weight: Option<u16>,   // 0-65535, default 0
}

RelationUpdate {
    op: UpdateOperation,   // Set | Remove
    name: String,          // Relationship name
    value: Option<IndexValue>, // Optional typed value
    address: Addr,         // Related address
}
```

#### `Configure`

Update factory configuration (manager only).

```rust
ExecuteMsg::Configure(Config {
    managed_by: Addr,
    default_code_id: Option<Uint64>,
    allowed_code_ids: Vec<Uint64>,
})
```

#### `Presets::Set`

Create or update a preset template (manager only).

```rust
ExecuteMsg::Presets(PresetsExecuteMsg::Set(SetPresetMsg {
    name: String,
    values: Map<String, Value>,
    overridable: bool,  // If true, user values override preset
}))
```

#### `Presets::Remove`

Delete a preset template (manager only).

```rust
ExecuteMsg::Presets(PresetsExecuteMsg::Remove {
    name: String
})
```

#### `Migrations::Migrate`

Migrate a single contract (manager only).

```rust
ExecuteMsg::Migrations(MigrationsExecuteMsg::Migrate(
    SingletonMigrationParams {
        contract: Addr,
        from_code_id: Option<Uint64>,  // Only migrate if matches
        to_code_id: Uint64,
        migrate_msg: Option<Binary>,
    }
))
```

#### `Migrations::Session::Begin`

Start a batch migration session (manager only).

```rust
ExecuteMsg::Migrations(MigrationsExecuteMsg::Session(
    MigrationSessionMsg::Begin(MigrationParams {
        name: String,                  // Unique session name
        from_code_id: Option<Uint64>,
        to_code_id: Uint64,
        migrate_msg: Option<Binary>,
        batch_size: Option<u16>,       // 1-100, default 50
        error_strategy: MigrationErrorStrategy, // Abort | Retry
    })
))
```

#### `Migrations::Session::Step`

Process next batch in migration session (manager only).

```rust
ExecuteMsg::Migrations(MigrationsExecuteMsg::Session(
    MigrationSessionMsg::Step { name: String }
))
```

#### `Migrations::Session::Retry`

Retry failed migrations in a session (manager only).

```rust
ExecuteMsg::Migrations(MigrationsExecuteMsg::Session(
    MigrationSessionMsg::Retry {
        name: String,
        params: Option<MigrationParams>, // Override params
    }
))
```

#### `Migrations::Session::Cancel`

Cancel and cleanup a migration session (manager only).

```rust
ExecuteMsg::Migrations(MigrationsExecuteMsg::Session(
    MigrationSessionMsg::Cancel { name: String }
))
```

### Query Messages

#### `Config`

Get factory configuration.

```rust
QueryMsg::Config {}
```

**Returns:** `ConfigResponse`

#### `Contracts::InRange`

Query contracts by index range.

```rust
QueryMsg::Contracts(ContractSetQueryMsg::InRange(
    ContractsInRangeQueryParams {
        index: IndexSelector,  // Which index to query
        start: Option<IndexRangeBound>,
        stop: Option<IndexRangeBound>,
        limit: Option<u16>,    // 1-500, default 100
        desc: Option<bool>,    // Descending order
        cursor: Option<(Vec<u8>, ContractId)>,
    }
))
```

**IndexSelector variants:**
- `CreatedBy` - Query by creator address
- `CreatedAt` - Query by creation timestamp
- `UpdatedAt` - Query by last update timestamp
- `CodeId` - Query by code ID
- `Admin` - Query by admin address
- `Tag` - Query contracts with a tag (any weight)
- `Custom(String)` - Query by custom index name

**Returns:** `ContractsByIndexResponse`

#### `Contracts::WithTag`

Query contracts by tag with weight filtering.

```rust
QueryMsg::Contracts(ContractSetQueryMsg::WithTag(
    ContractsByTagQueryParams {
        tag: String,
        min_weight: Option<TagWeightRangeBound>,
        max_weight: Option<TagWeightRangeBound>,
        limit: Option<u16>,
        desc: Option<bool>,
        cursor: Option<(Vec<u8>, u16, ContractId)>,
    }
))
```

**Returns:** `ContractsByTagResponse` (includes weights)

#### `Contracts::RelatedTo`

Query contracts related to an address.

```rust
QueryMsg::Contracts(ContractSetQueryMsg::RelatedTo(
    ContractsRelatedToParams {
        address: Addr,
        start: Option<RangeQueryBound<NameValue>>,
        stop: Option<RangeQueryBound<NameValue>>,
        limit: Option<u16>,
        desc: Option<bool>,
        cursor: Option<(ContractId, Vec<u8>)>,
    }
))
```

**Returns:** `ContractsRelatedToResponse` (includes relationship values)

#### `Contract::Metadata`

Get contract creation/update metadata.

```rust
QueryMsg::Contract(ContractQueryMsg::Metadata {
    address: Addr
})
```

**Returns:**
```rust
ContractMetadataResponse {
    created_at: Timestamp,
    created_by: Addr,
    updated_at: Timestamp,
    name: Option<String>,
    code_id: Uint64,
    admin: Addr,
}
```

#### `Contract::Tags`

List all tags for a contract.

```rust
QueryMsg::Contract(ContractQueryMsg::Tags(
    ContractTagsQueryParams {
        contract: Addr,
        start: Option<RangeQueryBound<String>>,
        stop: Option<RangeQueryBound<String>>,
        limit: Option<u16>,
        desc: Option<bool>,
        cursor: Option<String>,
    }
))
```

**Returns:** `ContractTagsResponse`

#### `Contract::HasTags`

Check if contract has specific tags.

```rust
QueryMsg::Contract(ContractQueryMsg::HasTags(
    ContractHasTagsQueryParams {
        address: Addr,
        test: BooleanTest,  // And | Or | Xor
        tags: Vec<TagSelector>,
    }
))
```

**Returns:** `bool`

#### `Contract::Relations`

List all relationships for a contract.

```rust
QueryMsg::Contract(ContractQueryMsg::Relations(
    ContractRelationsQueryParams {
        contract: Addr,
        start: Option<RangeQueryBound<NameValue>>,
        stop: Option<RangeQueryBound<NameValue>>,
        limit: Option<u16>,
        desc: Option<bool>,
        cursor: Option<(Vec<u8>, Addr)>,
    }
))
```

**Returns:** `ContractRelationsResponse`

#### `Contract::IsRelatedTo`

Check if contract has specific relationships.

```rust
QueryMsg::Contract(ContractQueryMsg::IsRelatedTo(
    ContractHasRelationsQueryParams {
        contract_address: Addr,
        address: Addr,
        test: BooleanTest,
        relations: Vec<NameValue>,
    }
))
```

**Returns:** `ContractIsRelatedToResponse`

#### `Presets::Get`

Get a preset by name.

```rust
QueryMsg::Presets(PresetsQueryMsg::Get {
    name: String
})
```

**Returns:** `PresetResponse`

#### `Presets::Paginate`

List all presets.

```rust
QueryMsg::Presets(PresetsQueryMsg::Paginate {
    cursor: Option<String>
})
```

**Returns:** `PresetPaginationResponse`

#### `Migrations::Session`

Get migration session status.

```rust
QueryMsg::Migrations(MigrationsQueryMsg::Session(String))
```

**Returns:** `MigrationSessionResponse`

## Advanced Features

### Custom Index Strategy

**When to use custom indexes:**
- Frequent queries by a specific field (e.g., `total_supply`, `price`)
- Range-based filtering needs (e.g., all contracts with supply > 1M)
- Sorting requirements beyond built-in indexes

**Index naming conventions:**
```rust
// Good: Descriptive, lowercase, underscore-separated
"total_supply", "last_trade_price", "vault_tvl"

// Avoid: Special characters, spaces, very long names
"Total Supply", "price$$$", "my_extremely_long_index_name_that_is_hard_to_remember"
```

**Performance considerations:**
- Each index adds ~2 storage writes per update
- String indexes are padded to 128 bytes (use sparingly)
- Numeric types (`Uint*`/`Int*`) are most efficient
- Limit to 5-10 custom indexes per contract for optimal performance

### Relationship Graph Patterns

**1. Parent-Child Hierarchies**
```rust
// DAO → SubDAOs
RelationUpdate {
    name: "sub_dao".to_string(),
    value: Some(IndexValue::Uint64(sub_dao_index)),
    address: sub_dao_addr,
}
```

**2. Many-to-Many Associations**
```rust
// Token ↔ Liquidity Pools
RelationUpdate {
    name: "liquidity_pool".to_string(),
    value: Some(IndexValue::String(pool_type)), // "stable" | "volatile"
    address: pool_addr,
}
```

**3. Dependency Tracking**
```rust
// Contract → Dependencies
RelationUpdate {
    name: "depends_on".to_string(),
    value: Some(IndexValue::String(version)), // "v1.2.3"
    address: dependency_addr,
}
```

### Migration Error Recovery

**Strategy: Retry**
- Failed migrations are tracked in `MIGRATION_ERRORS`
- Call `Retry` to re-attempt failed contracts
- Useful for transient errors (gas, timeouts)

**Strategy: Abort**
- Entire transaction reverts on first error
- Use for critical migrations requiring atomicity
- Allows manual intervention before proceeding

**Best practices:**
1. Test migration on devnet with small batch sizes
2. Use `Retry` strategy for large production migrations
3. Monitor `MigrationSessionResponse` for progress
4. Cancel and restart if error rate > 10%

### Preset Inheritance

**Overridable Presets** (`overridable: true`):
```
Final Values = Preset ∪ User Values
(User values take precedence)
```

**Locked Presets** (`overridable: false`):
```
Final Values = User Values ∪ Preset
(Preset values take precedence)
```

**Use cases:**
- **Overridable**: Templates with sensible defaults (decimals, marketing)
- **Locked**: Enforced parameters (oracle address, protocol fee)

### Gas Optimization Tips

1. **Batch Operations**: Group multiple updates in single transaction
2. **Cursor Pagination**: Always use cursors for large result sets
3. **Limit Indexes**: Only create indexes you actually query
4. **Numeric Types**: Prefer `Uint64` over `String` for indexes when possible
5. **Tag Weights**: Use sparse weights (0, 50, 100) vs dense (0, 1, 2, 3...)

## Building & Testing

### Prerequisites

- Rust 1.70+
- `wasm32-unknown-unknown` target
- Docker (for optimized builds)

### Build Locally

```bash
# Development build
cargo build

# Run unit tests
cargo test

# Generate schema files
cargo schema
```

### Optimized Build

```bash
# Using workspace optimizer (recommended)
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.15.0
```

Output: `artifacts/cw_factory.wasm` (~180KB optimized)

### Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --features library

# Coverage
cargo tarpaulin --out Html
```

### Schema Generation

```bash
cargo schema
# Output: JSON schema files for all messages and responses
```

## Deployment

### Deploy to Testnet

```bash
# 1. Store WASM
junod tx wasm store artifacts/cw_factory.wasm \
  --from <your-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.025ujuno \
  --chain-id <chain-id>

# 2. Get code ID from response

# 3. Instantiate
junod tx wasm instantiate <code-id> \
  '{"config":{"managed_by":"<manager-addr>","default_code_id":null,"allowed_code_ids":[]}}' \
  --from <your-key> \
  --label "CW Factory v1.0" \
  --admin <admin-addr> \
  --gas auto
```

### Mainnet Checklist

- [ ] Audit contract code
- [ ] Test all features on testnet
- [ ] Verify migration paths
- [ ] Document all custom indexes
- [ ] Set up monitoring for factory events
- [ ] Configure appropriate `allowed_code_ids`
- [ ] Test batch migration with small batch sizes
- [ ] Set up preset templates
- [ ] Document manager key security practices

## Performance & Security

### Performance Characteristics

| Operation | Estimated Gas | Notes |
|-----------|--------------|-------|
| Create Contract | ~500k | Includes instantiation + indexing |
| Update (1 index) | ~150k | Per index updated |
| Update (1 tag) | ~100k | Per tag |
| Update (1 relation) | ~120k | Per relation |
| Query (range, 100 items) | ~50k | Depends on result size |
| Migration (per contract) | ~200k | Varies by contract complexity |

*Estimates based on average CosmWasm gas costs*

### Security Considerations

**Access Control**

- **Manager** has elevated privileges (migrations, presets, manual updates)
- Use multi-sig or DAO governance for manager address
- Factory is admin of created contracts by default (required for migrations)

**Migration Safety**

- Always test migrations on subset of contracts first
- Use `from_code_id` filter to target specific versions
- Monitor error rates during batch migrations
- Keep batch sizes reasonable (50-100) to avoid gas limits

**Index Limits**

- Maximum string key size: 128 bytes
- Maximum tag weight: 65535
- Maximum query limit: 500 items per page
- Cursor-based pagination prevents DoS attacks

## FAQ

**Q: Can I use this factory with any CosmWasm contract?**
A: Yes, as long as the contract's code_id is in `allowed_code_ids`.

**Q: How do child contracts update their indexes?**
A: Via SubMsg callback to `ExecuteMsg::Update` with `contract: None`.

**Q: Can I query contracts created before adding a custom index?**
A: Only if contracts explicitly call `Update` to populate that index.

**Q: What happens if a migration fails mid-batch?**
A: With `Retry` strategy, failed contracts are tracked and can be retried. With `Abort`, the entire batch reverts.

**Q: Can I delete contracts from the factory?**
A: No, contracts remain indexed. Use tags (e.g., "archived") to mark inactive contracts.

**Q: How much storage does the factory use?**
A: ~200 bytes per contract + ~50 bytes per index + ~40 bytes per tag + ~60 bytes per relation.

**Q: Can multiple factories share the same code_id pool?**
A: Yes, each factory instance is independent.

**Q: What's the difference between tags and custom indexes?**
A: Tags are simple string labels with optional weights, ideal for categorization. Custom indexes support typed values and are better for numerical comparisons and ranges.

## License

Apache 2.0

## Acknowledgments

Built with [CosmWasm](https://cosmwasm.com/) and inspired by traditional database indexing patterns.
