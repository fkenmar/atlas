# ADR 0002 — Index-based adjacency for the symbol graph

## Context

The link and rank stages build a directed graph of file and symbol nodes (PRD §5.1). The natural object-graph design — nodes holding references to their neighbors — fights Rust's ownership model: cyclic references force `Rc<RefCell<…>>` or arena allocators with lifetimes threaded through every API, and the PRD explicitly names borrow-checker friction on graph structures as a project risk (§10). The graph's only heavy consumer is power-iteration PageRank, which wants fast sequential iteration over nodes and edges, not pointer-chasing. The graph is built once in link and read-only afterward; nodes are never deleted mid-run.

## Decision

Graph structures store nodes in a `Vec<Node>` and refer to them exclusively by `usize` index handles, with adjacency as `Vec<Vec<usize>>`. No references, lifetimes, `Rc`, `RefCell`, or any interior mutability appear in any graph data structure. Handles are plain numbers, stable for the life of one pipeline run because the graph is immutable after construction. This applies to `src/link.rs`, `src/rank.rs`, and anything else that holds graph data.

## Consequences

- No lifetime parameters anywhere in the graph code; structures are trivially `Send`, serializable for the cache, and cheap to move between pipeline stages — this is the standard Rust pattern for graphs.
- PageRank becomes tight loops over contiguous vectors: cache-friendly and easy to parallelize later if the serial tail matters (NFR-1).
- Indices are not checked by the borrow checker: a stale or out-of-range `usize` is a logic bug the compiler cannot catch. Mitigated by immutability-after-construction and by never letting indices escape a single pipeline run.
- Newtype handles (e.g. `NodeId(usize)`) can be layered on later for type safety without changing this design; that would be a code change, not a new ADR.
