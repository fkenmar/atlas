//! JSON renderer — stable, versioned schema for programmatic consumers
//! (sketch: PRD §7.3). Any change to the output shape bumps
//! `SCHEMA_VERSION` and, pre-1.0, the crate minor version (see the
//! release-process skill).

/// Version stamped into every JSON output as `"version"`.
pub const SCHEMA_VERSION: u32 = 1;

pub struct JsonRenderer;

impl super::Renderer for JsonRenderer {
    fn render(&self, _map: &crate::budget::BudgetedMap) -> String {
        todo!("M1: serde_json rendering of the versioned schema")
    }
}

#[cfg(test)]
mod tests {
    // Schema-stability tests land with the M1 implementation.
}
