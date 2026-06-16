//! atlas — compile a codebase into a token-budgeted structural map for LLM
//! coding agents.
//!
//! Pipeline: discover → parse → link → rank → budget → render. Each stage
//! lives in its own module; `cli` parses flags and drives the stages. The
//! library target exists so integration tests and benches can drive the
//! pipeline; the binary (`main.rs`) is a thin wrapper over [`cli::run`].

pub mod budget;
pub mod cache;
pub mod cli;
pub mod discover;
pub mod lang;
pub mod link;
pub mod parse;
pub mod rank;
pub mod render;
