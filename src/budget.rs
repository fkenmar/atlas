//! Stage 5 — budget: greedily pack ranked symbols into the token budget
//! using exact BPE token counts (tiktoken-rs, FR-11). Degradation ladder
//! (PRD §5.1): drop private symbols → drop parameter names (keep types) →
//! collapse files to one-line summaries → drop the file entirely. The
//! repo's directory skeleton is always retained.

/// What survived packing: the symbol selection and per-file degradation
/// level the renderers consume.
pub struct BudgetedMap {
    pub target_tokens: usize,
    pub rendered_tokens: usize,
}

/// Pack ranked symbols into `budget_tokens`, applying the degradation
/// ladder deterministically (G5: same inputs → byte-identical output).
pub fn pack(_ranking: &crate::rank::Ranking, _budget_tokens: usize) -> BudgetedMap {
    todo!("M1: greedy packing with the degradation ladder and exact token counts")
}

#[cfg(test)]
mod tests {
    // Packing-determinism and ladder-order tests land with the M1 implementation.
}
