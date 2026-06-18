//! atlas — compile a codebase into a token-budgeted structural map for LLM
//! coding agents.
//!
//! Pipeline: discover → parse → link → rank → budget → render. Each stage
//! lives in its own module; `cli` parses flags and drives the stages. The
//! binary (`main.rs`) is a thin wrapper over [`cli::run`].
//!
//! # Embedding atlas as a library
//!
//! Use [`api::build_map`] to produce a map from a path, then render it:
//!
//! ```no_run
//! use atlas::api::{build_map, MapOptions};
//! let map = build_map(std::path::Path::new("."), &MapOptions::default())?;
//! print!("{}", atlas::render::markdown::render(&map));
//! # Ok::<(), atlas::api::MapError>(())
//! ```
//!
//! [`api`] is the *supported* embedding surface; the per-stage modules below are
//! public for the binary/tests and may change between minor releases. See the
//! [`api`] docs for the semver policy.

pub mod api;
pub mod budget;
pub mod cache;
pub mod cli;
pub mod diff;
pub mod discover;
pub mod lang;
pub mod link;
pub mod mcp;
pub mod parse;
pub mod rank;
pub mod render;
