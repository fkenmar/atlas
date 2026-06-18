//! Minimal example of embedding atlas as a library (#69): map a directory and
//! print the Markdown map. Run with:
//!
//!     cargo run --example embed -- path/to/repo

use std::path::Path;

use atlas::api::{build_map, MapOptions};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    match build_map(Path::new(&path), &MapOptions::default()) {
        Ok(map) => print!("{}", atlas::render::markdown::render(&map)),
        Err(err) => {
            eprintln!("atlas: {err}");
            std::process::exit(1);
        }
    }
}
