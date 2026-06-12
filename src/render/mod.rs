//! Stage 6 — render: emit the budgeted map as Markdown (default, optimized
//! for LLM readability), JSON (versioned schema, PRD §7.3), or XML (M3).
//!
//! Determinism is a hard requirement here (G5/NFR-4): renderers iterate
//! sorted collections only — BTreeMap or an explicit sort before emit,
//! never HashMap iteration order.

pub mod json;
pub mod markdown;

pub trait Renderer {
    fn render(&self, map: &crate::budget::BudgetedMap) -> String;
}

#[cfg(test)]
mod tests {
    // Byte-identical-output (determinism) tests land with the M1 renderers.
}
