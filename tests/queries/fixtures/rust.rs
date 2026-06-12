//! Fixture for queries/rust/tags.scm — one construct per extraction rule.
//! Not compiled by cargo (lives below tests/, not at tests/*.rs); it only
//! needs to parse.

use std::collections::BTreeMap;

pub const API_VERSION: &str = "1.0";
pub static GLOBAL_FLAG: bool = false;

pub fn top_level(x: u32) -> u32 {
    helper(x)
}

fn helper(x: u32) -> u32 {
    x + 1
}

pub struct Service {
    names: BTreeMap<String, u32>,
}

pub enum Level {
    Low,
    High,
}

pub trait Runner {
    fn run(&self);

    fn ready(&self) -> bool {
        true
    }
}

impl Service {
    pub fn new() -> Self {
        Self {
            names: BTreeMap::new(),
        }
    }
}

impl Runner for Service {
    fn run(&self) {
        let _ = self.names.len();
        let _ = String::from("scoped call");
    }
}

pub mod nested {
    pub type Alias = u64;
}

macro_rules! shout {
    ($x:expr) => {
        $x
    };
}
