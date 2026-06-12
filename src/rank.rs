//! Stage 4 — rank: personalized PageRank over the import/reference graph —
//! the approach Aider validated (PRD §5.1). In-house power iteration:
//! damping 0.85, 20 iterations, convergence check (PRD §7.2). `--focus`
//! paths seed the personalization vector so the map adapts to the task.

/// PageRank scores, indexed parallel to `Graph::nodes`.
pub struct Ranking(pub Vec<f64>);

/// Rank every node. `focus` holds node indices boosted by the
/// personalization vector (files passed via `--focus`); empty = uniform.
pub fn rank(_graph: &crate::link::Graph, _focus: &[usize]) -> Ranking {
    todo!("M1: power-iteration personalized PageRank")
}

#[cfg(test)]
mod tests {
    // Convergence and personalization tests land with the M1 implementation.
}
