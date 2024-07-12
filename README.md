# CosmWasm Factory
## Abstract
Chain-agnostic CosmWasm smart contract that implements the "factory" AKA
"repository" pattern. Devs or dApps may instantiate other smart contacts via the
factory's `create` method. These contracts are then indexed by creator, creation
timestamp, code ID, admin, and a host of other things. It also supports custom
indexes of arbitrary types. It is expected that these indexes are updated
explicitly via SubMsgs from the contracts instantiated through and managed by
the factory, via the `update` method. Alternatively the "manager" of the factory
may apply updates as well. 

## Smart Queries
Beyond creation and update, the contract provides smart queries that are
instrumental to any front end, allowing efficient pagination of contract
addresses by scanning any existing indices. Aside from indices, the factory also
supports the ability to apply weighted tags to contracts as well as the ability
to create arbitary relationships between its contract addresses and other
addresses or strings, where each relationship id defined by a name and optional
associated value, i.e. `contractAddr <- {name: value} -> arbitraryAddrOrString`.