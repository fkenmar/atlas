//! Inspect tree-sitter node names for a source file.
//!
//! Usage: `cargo run --example dump-ast <file>`
//!
//! The grammar-engineer's tool for finding capture targets when writing
//! `queries/<lang>/tags.scm`: parses `<file>` with the grammar matching its
//! extension and pretty-prints the named AST — node kind, field name, span,
//! and leaf text. Languages without a wired grammar (TS/JS and Rust until
//! M1) get a clear notice instead.

use std::process::ExitCode;

use repomap::lang::Language;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example dump-ast <file>");
        return ExitCode::FAILURE;
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("dump-ast: cannot read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let lang = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .and_then(Language::from_extension);
    let Some(lang) = lang else {
        eprintln!("dump-ast: {path} is not a Tier 1 language file (py/ts/js/rs)");
        return ExitCode::FAILURE;
    };
    let Some(grammar) = lang.grammar() else {
        eprintln!(
            "dump-ast: the {} grammar isn't wired yet (lands in M1); python works today.",
            lang.name()
        );
        return ExitCode::FAILURE;
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&grammar).is_err() {
        eprintln!(
            "dump-ast: grammar/library version mismatch for {}",
            lang.name()
        );
        return ExitCode::FAILURE;
    }
    let Some(tree) = parser.parse(&source, None) else {
        eprintln!("dump-ast: tree-sitter could not parse {path}");
        return ExitCode::FAILURE;
    };

    print_node(tree.root_node(), &source, 0, None);
    ExitCode::SUCCESS
}

fn print_node(node: tree_sitter::Node, source: &str, depth: usize, field: Option<&str>) {
    if node.is_named() {
        let indent = "  ".repeat(depth);
        let field_label = field.map(|f| format!("{f}: ")).unwrap_or_default();
        let start = node.start_position();
        let end = node.end_position();
        let mut line = format!(
            "{indent}{field_label}({}) [{}:{} - {}:{}]",
            node.kind(),
            start.row + 1,
            start.column,
            end.row + 1,
            end.column
        );
        if node.named_child_count() == 0 {
            let text = source.get(node.byte_range()).unwrap_or("");
            let excerpt: String = text.chars().take(40).collect();
            line.push_str(&format!("  {excerpt:?}"));
        }
        println!("{line}");
    }
    let depth = if node.is_named() { depth + 1 } else { depth };
    let mut cursor = node.walk();
    for (i, child) in node.children(&mut cursor).enumerate() {
        let field = node.field_name_for_child(i as u32);
        print_node(child, source, depth, field);
    }
}
