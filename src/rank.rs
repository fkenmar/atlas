//! Stage 4 — rank: personalized PageRank over the import/reference graph —
//! the approach Aider validated (PRD §5.1). In-house power iteration:
//! damping 0.85, up to 20 iterations with an L1 convergence check (PRD §7.2).
//! `--focus` node indices seed the personalization vector so the map adapts
//! to the task.
//!
//! Dangling nodes (Symbol nodes are sinks in the link graph) have their rank
//! mass redistributed through the personalization vector each iteration, so
//! the score vector stays a proper distribution summing to 1. All arithmetic
//! runs in node-index order — deterministic across runs and platforms (NFR-4).

use crate::link::Graph;

/// Damping factor: probability of following an edge vs. teleporting (PRD §7.2).
const DAMPING: f64 = 0.85;
/// Iteration cap; power iteration usually converges well before this.
const MAX_ITERS: usize = 20;
/// Stop early once the total (L1) change across all nodes drops below this.
const EPSILON: f64 = 1e-9;

/// PageRank scores, indexed parallel to `Graph::nodes`. Scores sum to ~1.
pub struct Ranking(pub Vec<f64>);

impl Ranking {
    /// Score of node `i` (0.0 if out of range).
    pub fn score(&self, i: usize) -> f64 {
        self.0.get(i).copied().unwrap_or(0.0)
    }
}

/// Rank every node. `focus` holds node indices boosted by the personalization
/// vector (files passed via `--focus`); empty (or all-invalid) = uniform
/// teleport, i.e. ordinary PageRank.
pub fn rank(graph: &Graph, focus: &[usize]) -> Ranking {
    let n = graph.nodes.len();
    if n == 0 {
        return Ranking(Vec::new());
    }

    // Personalization / teleport vector p (sums to 1).
    let p = personalization(n, focus);

    // Power iteration.
    let mut rank = p.clone();
    for _ in 0..MAX_ITERS {
        let mut next = vec![0.0f64; n];
        let mut dangling = 0.0;
        for (i, adj) in graph.edges.iter().enumerate() {
            if adj.is_empty() {
                dangling += rank[i]; // redistributed via p below
            } else {
                let share = rank[i] / adj.len() as f64;
                for &j in adj {
                    next[j] += share;
                }
            }
        }
        let mut delta = 0.0;
        for i in 0..n {
            let v = (1.0 - DAMPING) * p[i] + DAMPING * (next[i] + dangling * p[i]);
            delta += (v - rank[i]).abs();
            next[i] = v;
        }
        rank = next;
        if delta < EPSILON {
            break;
        }
    }
    Ranking(rank)
}

/// Build the teleport distribution: uniform over all nodes, or — when one or
/// more in-range focus indices are given — uniform over just those.
fn personalization(n: usize, focus: &[usize]) -> Vec<f64> {
    let mut seeds: Vec<usize> = focus.iter().copied().filter(|&i| i < n).collect();
    seeds.sort_unstable();
    seeds.dedup();

    let mut p = vec![0.0f64; n];
    if seeds.is_empty() {
        p.fill(1.0 / n as f64);
    } else {
        let share = 1.0 / seeds.len() as f64;
        for i in seeds {
            p[i] = share;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Graph, Node, NodeKind};

    fn file_node(label: &str, idx: usize) -> Node {
        Node {
            kind: NodeKind::File,
            label: label.to_string(),
            file: idx,
            symbol: None,
        }
    }

    fn sym_node(label: &str, file: usize) -> Node {
        Node {
            kind: NodeKind::Symbol,
            label: label.to_string(),
            file,
            symbol: Some(0),
        }
    }

    fn sum(r: &Ranking) -> f64 {
        r.0.iter().sum()
    }

    #[test]
    fn empty_graph_returns_empty() {
        let g = Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        assert!(rank(&g, &[]).0.is_empty());
    }

    #[test]
    fn scores_form_a_distribution() {
        // f0, f1, f2, symA(f0); f1->symA, f2->symA.
        let g = Graph {
            nodes: vec![
                file_node("f0", 0),
                file_node("f1", 1),
                file_node("f2", 2),
                sym_node("A", 0),
            ],
            edges: vec![vec![], vec![3], vec![3], vec![]],
        };
        let r = rank(&g, &[]);
        assert!(
            (sum(&r) - 1.0).abs() < 1e-9,
            "scores must sum to 1, got {}",
            sum(&r)
        );
        assert!(r.0.iter().all(|&s| s >= 0.0));
    }

    #[test]
    fn referenced_symbol_outranks_unreferenced() {
        // symA (node 3) referenced by f1 and f2; symB (node 4) referenced by
        // nobody. A must outrank B.
        let g = Graph {
            nodes: vec![
                file_node("f0", 0),
                file_node("f1", 1),
                file_node("f2", 2),
                sym_node("A", 0),
                sym_node("B", 1),
            ],
            edges: vec![vec![], vec![3], vec![3], vec![], vec![]],
        };
        let r = rank(&g, &[]);
        assert!(
            r.score(3) > r.score(4),
            "referenced symbol A ({}) should outrank unreferenced B ({})",
            r.score(3),
            r.score(4)
        );
    }

    #[test]
    fn more_referenced_symbol_ranks_higher() {
        // symA referenced 2x, symB referenced 1x → A > B.
        let g = Graph {
            nodes: vec![
                file_node("f0", 0),
                file_node("f1", 1),
                file_node("f2", 2),
                sym_node("A", 0),
                sym_node("B", 0),
            ],
            // f1->A, f2->A, f1->B
            edges: vec![vec![], vec![3, 4], vec![3], vec![], vec![]],
        };
        let r = rank(&g, &[]);
        assert!(r.score(3) > r.score(4));
    }

    #[test]
    fn focus_boosts_the_focused_node() {
        // Two unconnected files; focusing f1 must lift its score above f0's.
        let g = Graph {
            nodes: vec![file_node("f0", 0), file_node("f1", 1)],
            edges: vec![vec![], vec![]],
        };
        let uniform = rank(&g, &[]);
        assert!((uniform.score(0) - uniform.score(1)).abs() < 1e-12);

        let focused = rank(&g, &[1]);
        assert!(
            focused.score(1) > focused.score(0),
            "focused f1 ({}) should outrank f0 ({})",
            focused.score(1),
            focused.score(0)
        );
        assert!((sum(&focused) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn out_of_range_focus_falls_back_to_uniform() {
        let g = Graph {
            nodes: vec![file_node("f0", 0), file_node("f1", 1)],
            edges: vec![vec![], vec![]],
        };
        let r = rank(&g, &[99]); // no valid focus → uniform
        assert!((r.score(0) - r.score(1)).abs() < 1e-12);
    }

    #[test]
    fn is_deterministic() {
        let g = Graph {
            nodes: vec![file_node("f0", 0), file_node("f1", 1), sym_node("A", 0)],
            edges: vec![vec![], vec![2], vec![]],
        };
        let a = rank(&g, &[]);
        let b = rank(&g, &[]);
        assert_eq!(a.0, b.0);
    }
}
