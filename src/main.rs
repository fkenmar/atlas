//! atlas binary entry point. All logic lives in the library crate so
//! integration tests and benches can drive the pipeline directly.

fn main() {
    atlas::cli::run();
}
