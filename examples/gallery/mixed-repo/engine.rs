//! Core scoring engine (Rust).

pub struct Engine {
    weights: Vec<f64>,
}

impl Engine {
    pub fn new(weights: Vec<f64>) -> Engine {
        Engine { weights }
    }

    /// Weighted dot product of `features` against the configured weights.
    pub fn score(&self, features: &[f64]) -> f64 {
        self.weights.iter().zip(features).map(|(w, f)| w * f).sum()
    }

    pub fn rank(&self, rows: &[Vec<f64>]) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..rows.len()).collect();
        idx.sort_by(|&a, &b| {
            self.score(&rows[b])
                .partial_cmp(&self.score(&rows[a]))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        idx
    }
}

pub fn default_engine() -> Engine {
    Engine::new(vec![0.5, 0.3, 0.2])
}
