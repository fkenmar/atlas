//! MCP server (#7, ADR 0008): exposes the atlas map to an LLM agent as a tool
//! call over the stdio JSON-RPC 2.0 transport (newline-delimited messages).
//!
//! [`handle`] is a pure request → response dispatch (`None` for notifications),
//! so the protocol is unit-testable without any I/O; [`serve`] is a thin stdin
//! loop over it. One tool, `get_map`, runs the map pipeline via [`build_map`].

use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::{json, Value};

use crate::budget::{pack, BudgetOptions, BudgetedMap, DEFAULT_BUDGET};

/// The MCP protocol revision this server speaks.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server: one JSON-RPC message per line in, one per line out.
pub fn serve() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle(&req),
            Err(_) => Some(error(Value::Null, -32700, "Parse error")),
        };
        if let Some(response) = response {
            writeln!(
                out,
                "{}",
                serde_json::to_string(&response).unwrap_or_default()
            )?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Dispatch one JSON-RPC request to its response, or `None` for a notification.
pub fn handle(req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let is_notification = req.get("id").is_none();

    match method {
        "initialize" => Some(result(id, initialize_result())),
        "tools/list" => Some(result(id, json!({ "tools": [get_map_tool()] }))),
        "tools/call" => Some(result(id, handle_tools_call(req))),
        "ping" => Some(result(id, json!({}))),
        // Notifications (no `id`) get no response — e.g. notifications/initialized.
        _ if is_notification => None,
        _ => Some(error(id, -32601, "Method not found")),
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "atlas", "version": env!("CARGO_PKG_VERSION") },
    })
}

fn get_map_tool() -> Value {
    json!({
        "name": "get_map",
        "description": "Compile a codebase into a token-budgeted structural map \
                        (signatures, types, import edges; no function bodies) for \
                        navigating a repo without reading every file.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository root to map." },
                "budget": { "type": "integer", "description": "Token budget (default 2048)." },
                "no_private": { "type": "boolean", "description": "Public API surface only." },
                "format": { "type": "string", "enum": ["md", "json", "xml"], "description": "Output format (default md)." }
            },
            "required": ["path"]
        }
    })
}

/// Execute a `tools/call`. Tool-level failures come back as an `isError` result
/// (so the model sees them); only the shape is a successful JSON-RPC result.
fn handle_tools_call(req: &Value) -> Value {
    let params = req.get("params");
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if name != "get_map" {
        return tool_error(format!(
            "unknown tool: {name:?} (only \"get_map\" is available)"
        ));
    }
    let args = params.and_then(|p| p.get("arguments"));
    let Some(path) = args.and_then(|a| a.get("path")).and_then(Value::as_str) else {
        return tool_error("get_map requires a string \"path\" argument".to_string());
    };
    let budget = args
        .and_then(|a| a.get("budget"))
        .and_then(Value::as_u64)
        .map(|b| b as usize)
        .unwrap_or(DEFAULT_BUDGET);
    let no_private = args
        .and_then(|a| a.get("no_private"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let format = args
        .and_then(|a| a.get("format"))
        .and_then(Value::as_str)
        .unwrap_or("md");

    match render_map(Path::new(path), budget, no_private, format) {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(e) => tool_error(e),
    }
}

fn render_map(
    path: &Path,
    budget: usize,
    no_private: bool,
    format: &str,
) -> Result<String, String> {
    if budget == 0 {
        return Err("budget must be at least 1 token".to_string());
    }
    let root = path
        .canonicalize()
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    if !root.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }
    let map = build_map(&root, budget, no_private)?;
    Ok(match format {
        "json" => crate::render::json::render(&map),
        "xml" => crate::render::xml::render(&map),
        "md" => crate::render::markdown::render(&map),
        other => return Err(format!("unknown format {other:?} (md, json, or xml)")),
    })
}

/// Run discover → parse → link → rank → budget for `root` (no `--focus`). The
/// CLI's `run_with` keeps its own copy of this pipeline for now (ADR 0008).
fn build_map(root: &Path, budget: usize, no_private: bool) -> Result<BudgetedMap, String> {
    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    let files = crate::discover::discover(root);
    if files.is_empty() {
        return Err(format!(
            "no supported source files found under {}",
            root.display()
        ));
    }
    let mut cache = crate::cache::Cache::open(root);
    let outcome = crate::parse::parse_all_cached(files, &mut cache);
    cache.save();
    let graph = crate::link::link(&outcome.files);
    let ranking = crate::rank::rank(&graph, &[]);
    let counter = crate::budget::TiktokenCounter::cl100k()
        .map_err(|e| format!("could not initialize the tokenizer: {e}"))?;
    let opts = BudgetOptions {
        budget_tokens: budget,
        no_private,
    };
    Ok(pack(
        &outcome.files,
        &graph,
        &ranking,
        &repo_name,
        outcome.stats,
        &opts,
        &counter,
    ))
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tool_error(message: String) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_source_repo(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("atlas-mcp-{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp repo");
        fs::write(dir.join("app.py"), "def run(value):\n    return value\n").expect("write source");
        dir
    }

    #[test]
    fn initialize_advertises_tools_and_server() {
        let resp = handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize"})).unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(resp["result"]["serverInfo"]["name"], "atlas");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_has_get_map() {
        let resp = handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
        assert_eq!(resp["result"]["tools"][0]["name"], "get_map");
        assert_eq!(
            resp["result"]["tools"][0]["inputSchema"]["required"][0],
            "path"
        );
    }

    #[test]
    fn ping_returns_empty_result() {
        let resp = handle(&json!({"jsonrpc":"2.0","id":3,"method":"ping"})).unwrap();
        assert!(resp["result"].is_object());
        assert!(resp.get("error").is_none());
    }

    #[test]
    fn notification_gets_no_response() {
        // No `id` → a notification → no reply.
        assert!(handle(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
    }

    #[test]
    fn unknown_method_is_jsonrpc_error() {
        let resp = handle(&json!({"jsonrpc":"2.0","id":4,"method":"does/not/exist"})).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn get_map_missing_path_is_tool_error() {
        let resp = handle(&json!({
            "jsonrpc":"2.0","id":5,"method":"tools/call",
            "params": { "name": "get_map", "arguments": {} }
        }))
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn get_map_unknown_tool_is_tool_error() {
        let resp = handle(&json!({
            "jsonrpc":"2.0","id":6,"method":"tools/call",
            "params": { "name": "nope", "arguments": {} }
        }))
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn get_map_renders_a_real_tree() {
        let repo = temp_source_repo("map");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(&json!({
            "jsonrpc":"2.0","id":7,"method":"tools/call",
            "params": {
                "name": "get_map",
                "arguments": { "path": path, "budget": 1024 }
            }
        }))
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("# atlas:"), "{text}");
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn get_map_json_format() {
        let repo = temp_source_repo("json");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(&json!({
            "jsonrpc":"2.0","id":8,"method":"tools/call",
            "params": {
                "name": "get_map",
                "arguments": { "path": path, "format": "json" }
            }
        }))
        .unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"version\":"), "{text}");
        let _ = fs::remove_dir_all(repo);
    }
}
