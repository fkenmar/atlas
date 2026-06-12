//! Stage 3 — link: resolve imports and defs↔refs into a directed graph of
//! file and symbol nodes. Resolution is best-effort and syntactic (module
//! paths, relative imports) — no type checker (PRD §5.1).
//!
//! The graph is index-based per ADR-0002: nodes live in a `Vec`, edges hold
//! `usize` handles. No references, lifetimes, `Rc`, or interior mutability
//! in graph structures.

pub struct Graph {
    pub nodes: Vec<Node>,
    /// Outgoing adjacency: `edges[i]` lists the node indices that node `i`
    /// points to (imports / references).
    pub edges: Vec<Vec<usize>>,
}

pub struct Node {
    pub kind: NodeKind,
    /// File path or qualified symbol name.
    pub label: String,
}

pub enum NodeKind {
    File,
    Symbol,
}

/// Build the import/reference graph from all parsed files.
pub fn link(_files: &[(crate::discover::SourceFile, crate::parse::ParsedFile)]) -> Graph {
    todo!("M1: best-effort syntactic import/reference resolution")
}

#[cfg(test)]
mod tests {
    // Resolution tests (relative imports, unresolvable refs) land in M1.
}
