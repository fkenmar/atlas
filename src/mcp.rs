//! MCP server (#7, ADR 0008): exposes the atlas map to an LLM agent as a tool
//! call over the stdio JSON-RPC 2.0 transport (newline-delimited messages).
//!
//! [`handle`] is a pure request → response dispatch (`None` for notifications),
//! so the protocol is unit-testable without any I/O; [`serve`] is a thin stdin
//! loop over it. Tools expose maps, symbol search, a thin anchor index, and
//! single-anchor expansion. All are strictly read-only — they parse uncached
//! (never writing a `.atlas` cache into the target), are confined to `root`
//! (#102), and make no network calls.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::budget::{BudgetedMap, DEFAULT_BUDGET};

/// The MCP protocol revision this server speaks.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server: one JSON-RPC message per line in, one per line out.
/// `root` confines every `get_map` to that subtree (#102) — a request for a path
/// outside it is rejected.
pub fn serve(root: &Path) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle(&req, root),
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
/// `root` is the confinement root for `get_map` (#102).
pub fn handle(req: &Value, root: &Path) -> Option<Value> {
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let is_notification = req.get("id").is_none();

    match method {
        "initialize" => Some(result(id, initialize_result())),
        "tools/list" => Some(result(
            id,
            json!({ "tools": [get_map_tool(), get_symbol_tool(), get_symbol_index_tool(), expand_symbol_tool()] }),
        )),
        "tools/call" => Some(result(id, handle_tools_call(req, root))),
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
                        navigating a repo without reading every file. Read-only; \
                        makes no network calls.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository root to map." },
                "budget": { "type": "integer", "description": "Token budget (default 2048)." },
                "no_private": { "type": "boolean", "description": "Public API surface only." },
                "format": { "type": "string", "enum": ["md", "json", "xml"], "description": "Output format (default md)." },
                "lang": { "type": "array", "items": { "type": "string" }, "description": "Restrict to languages by extension, e.g. [\"py\", \"rs\"]. A comma-separated string is also accepted." },
                "focus": { "type": "array", "items": { "type": "string" }, "description": "Boost these paths (files or directory prefixes, relative to path) in the ranking." }
            },
            "required": ["path"]
        }
    })
}

fn get_symbol_tool() -> Value {
    json!({
        "name": "get_symbol",
        "description": "Locate every definition of a symbol by exact name across the \
                        repo — each hit's file, line, kind, signature, and visibility. \
                        Answers \"where is X defined?\" for navigation, including \
                        multi-site names. Read-only; makes no network calls.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository root to search." },
                "name": { "type": "string", "description": "Exact symbol name to locate." },
                "no_private": { "type": "boolean", "description": "Public symbols only." },
                "lang": { "type": "array", "items": { "type": "string" }, "description": "Restrict to languages by extension, e.g. [\"py\", \"rs\"]. A comma-separated string is also accepted." }
            },
            "required": ["path", "name"]
        }
    })
}

fn get_symbol_index_tool() -> Value {
    json!({
        "name": "get_symbol_index",
        "description": "Return a thin, budgeted progressive-disclosure index of stable \
                        symbol anchors (relpath#name, with @line only for same-file \
                        duplicate names). Contains names, kinds, files, lines, and \
                        visibility, but no signatures or bodies; call expand_symbol \
                        for just-in-time detail.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository root to index." },
                "budget": { "type": "integer", "description": "Token budget for the rendered anchor index (default 2048)." },
                "no_private": { "type": "boolean", "description": "Public symbols only." },
                "lang": { "type": "array", "items": { "type": "string" }, "description": "Restrict to languages by extension, e.g. [\"py\", \"rs\"]. A comma-separated string is also accepted." }
            },
            "required": ["path"]
        }
    })
}

fn expand_symbol_tool() -> Value {
    json!({
        "name": "expand_symbol",
        "description": "Expand one symbol anchor from a thin map index into its full \
                        signature plus its defining file's one-hop neighbors (the files it \
                        imports and the files that import it). Pass an anchor like \
                        \"src/cache.rs#Cache\", or \"src/x.rs#from@42\" to pin an overload \
                        by line. Read-only and structural only — it never resolves who calls \
                        the symbol (that would be code intelligence).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository root the anchor is relative to." },
                "anchor": { "type": "string", "description": "Symbol anchor: \"relpath#name\" or \"relpath#name@line\"." },
                "no_private": { "type": "boolean", "description": "Resolve public symbols only." },
                "lang": { "type": "array", "items": { "type": "string" }, "description": "Restrict to languages by extension, e.g. [\"py\", \"rs\"]." }
            },
            "required": ["path", "anchor"]
        }
    })
}

/// Execute a `tools/call`. Tool-level failures come back as an `isError` result
/// (so the model sees them); only the shape is a successful JSON-RPC result.
/// `root` confines every tool to its subtree (#102).
fn handle_tools_call(req: &Value, root: &Path) -> Value {
    let params = req.get("params");
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let args = params.and_then(|p| p.get("arguments"));
    match name {
        "get_map" => call_get_map(args, root),
        "get_symbol" => call_get_symbol(args, root),
        "get_symbol_index" => call_get_symbol_index(args, root),
        "expand_symbol" => call_expand_symbol(args, root),
        other => tool_error(format!(
            "unknown tool: {other:?} (available: \"get_map\", \"get_symbol\", \"get_symbol_index\", \"expand_symbol\")"
        )),
    }
}

fn call_get_map(args: Option<&Value>, root: &Path) -> Value {
    let Some(path) = arg_str(args, "path") else {
        return tool_error("get_map requires a string \"path\" argument".to_string());
    };
    let budget = arg_u64(args, "budget")
        .map(|b| b as usize)
        .unwrap_or(DEFAULT_BUDGET);
    let no_private = arg_bool(args, "no_private");
    let format = arg_str(args, "format").unwrap_or("md");
    let langs = match parse_lang_arg(args) {
        Ok(l) => l,
        Err(e) => return tool_error(e),
    };
    let focus = parse_focus_arg(args);

    match render_map(path, root, budget, no_private, format, langs, focus) {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(e) => tool_error(e),
    }
}

fn call_get_symbol(args: Option<&Value>, root: &Path) -> Value {
    let Some(path) = arg_str(args, "path") else {
        return tool_error("get_symbol requires a string \"path\" argument".to_string());
    };
    let Some(name) = arg_str(args, "name") else {
        return tool_error("get_symbol requires a string \"name\" argument".to_string());
    };
    let no_private = arg_bool(args, "no_private");
    let langs = match parse_lang_arg(args) {
        Ok(l) => l,
        Err(e) => return tool_error(e),
    };
    let target = match confine(path, root) {
        Ok(t) => t,
        Err(e) => return tool_error(e),
    };
    let opts = crate::api::MapOptions {
        budget: DEFAULT_BUDGET,
        no_private,
        langs,
        focus: Vec::new(),
        cache: false,
    };
    match crate::api::find_symbol(&target, name, &opts) {
        Ok(hits) => {
            let definitions: Vec<Value> = hits
                .iter()
                .map(|h| {
                    json!({
                        "file": h.file,
                        "line": h.line,
                        "kind": h.kind,
                        "visibility": h.visibility,
                        "signature": h.signature,
                    })
                })
                .collect();
            let payload = json!({
                "name": name,
                "count": definitions.len(),
                "definitions": definitions,
            });
            let text = serde_json::to_string_pretty(&payload).unwrap_or_default();
            json!({ "content": [{ "type": "text", "text": text }], "isError": false })
        }
        Err(e) => tool_error(e.to_string()),
    }
}

fn call_get_symbol_index(args: Option<&Value>, root: &Path) -> Value {
    let Some(path) = arg_str(args, "path") else {
        return tool_error("get_symbol_index requires a string \"path\" argument".to_string());
    };
    let budget = arg_u64(args, "budget")
        .map(|b| b as usize)
        .unwrap_or(DEFAULT_BUDGET);
    let no_private = arg_bool(args, "no_private");
    let langs = match parse_lang_arg(args) {
        Ok(l) => l,
        Err(e) => return tool_error(e),
    };
    let target = match confine(path, root) {
        Ok(t) => t,
        Err(e) => return tool_error(e),
    };
    let opts = crate::api::MapOptions {
        budget,
        no_private,
        langs,
        focus: Vec::new(),
        cache: false,
    };
    match crate::api::build_symbol_index(&target, &opts) {
        Ok(index) => {
            let entries: Vec<Value> = index
                .entries
                .iter()
                .map(|e| {
                    json!({
                        "anchor": e.anchor,
                        "name": e.name,
                        "kind": e.kind,
                        "file": e.file,
                        "line": e.line,
                        "visibility": e.visibility,
                    })
                })
                .collect();
            let payload = json!({
                "repo": {
                    "name": index.repo_name,
                    "loc": index.total_loc,
                    "files": index.total_files,
                },
                "budget": {
                    "target": index.target_tokens,
                    "rendered": index.rendered_tokens,
                },
                "count": entries.len(),
                "entries": entries,
                "skipped_files": index.skipped_files,
                "unwired_files": index.unwired_files,
            });
            let text = serde_json::to_string_pretty(&payload).unwrap_or_default();
            json!({ "content": [{ "type": "text", "text": text }], "isError": false })
        }
        Err(e) => tool_error(e.to_string()),
    }
}

/// Parse a symbol anchor (ADR 0009): `relpath#name` or `relpath#name@line`.
/// Splits on the first `#` (file paths don't contain `#`); a trailing
/// `@<digits>` is the optional line pin. Returns `None` if either side is empty.
fn parse_anchor(anchor: &str) -> Option<(&str, &str, Option<usize>)> {
    let (file, rest) = anchor.split_once('#')?;
    if file.is_empty() || rest.is_empty() {
        return None;
    }
    if let Some((name, line_str)) = rest.rsplit_once('@') {
        if !name.is_empty() && !line_str.is_empty() && line_str.bytes().all(|b| b.is_ascii_digit())
        {
            if let Ok(line) = line_str.parse::<usize>() {
                return Some((file, name, Some(line)));
            }
        }
    }
    Some((file, rest, None))
}

fn call_expand_symbol(args: Option<&Value>, root: &Path) -> Value {
    let Some(path) = arg_str(args, "path") else {
        return tool_error("expand_symbol requires a string \"path\" argument".to_string());
    };
    let Some(anchor) = arg_str(args, "anchor") else {
        return tool_error(
            "expand_symbol requires a string \"anchor\" argument (e.g. \"src/cache.rs#Cache\")"
                .to_string(),
        );
    };
    let Some((file, name, line)) = parse_anchor(anchor) else {
        return tool_error(format!(
            "malformed anchor {anchor:?} — expected \"relpath#name\" or \"relpath#name@line\""
        ));
    };
    let no_private = arg_bool(args, "no_private");
    let langs = match parse_lang_arg(args) {
        Ok(l) => l,
        Err(e) => return tool_error(e),
    };
    let target = match confine(path, root) {
        Ok(t) => t,
        Err(e) => return tool_error(e),
    };
    let opts = crate::api::MapOptions {
        budget: DEFAULT_BUDGET,
        no_private,
        langs,
        focus: Vec::new(),
        cache: false,
    };
    match crate::api::expand_symbol(&target, file, name, line, &opts) {
        Ok(Some(e)) => {
            let payload = json!({
                "anchor": anchor,
                "found": true,
                "name": e.hit.name,
                "file": e.hit.file,
                "line": e.hit.line,
                "kind": e.hit.kind,
                "visibility": e.hit.visibility,
                "signature": e.hit.signature,
                "imports": e.imports,
                "used_by": e.used_by,
            });
            let text = serde_json::to_string_pretty(&payload).unwrap_or_default();
            json!({ "content": [{ "type": "text", "text": text }], "isError": false })
        }
        // A stale or unknown anchor is a successful empty result, not an error
        // (mirrors get_symbol's unknown-name behavior).
        Ok(None) => {
            let payload = json!({ "anchor": anchor, "found": false });
            let text = serde_json::to_string_pretty(&payload).unwrap_or_default();
            json!({ "content": [{ "type": "text", "text": text }], "isError": false })
        }
        Err(e) => tool_error(e.to_string()),
    }
}

fn render_map(
    requested: &str,
    root: &Path,
    budget: usize,
    no_private: bool,
    format: &str,
    langs: Vec<crate::lang::Language>,
    focus: Vec<String>,
) -> Result<String, String> {
    if budget == 0 {
        return Err("budget must be at least 1 token".to_string());
    }
    let target = confine(requested, root)?;
    let map = build_map(&target, budget, no_private, langs, focus)?;
    Ok(match format {
        "json" => crate::render::json::render(&map),
        "xml" => crate::render::xml::render(&map),
        "md" => crate::render::markdown::render(&map),
        other => return Err(format!("unknown format {other:?} (md, json, or xml)")),
    })
}

fn arg_str<'a>(args: Option<&'a Value>, key: &str) -> Option<&'a str> {
    args.and_then(|a| a.get(key)).and_then(Value::as_str)
}

fn arg_bool(args: Option<&Value>, key: &str) -> bool {
    args.and_then(|a| a.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn arg_u64(args: Option<&Value>, key: &str) -> Option<u64> {
    args.and_then(|a| a.get(key)).and_then(Value::as_u64)
}

/// Parse the optional `lang` argument: an array of extension strings, or a
/// single comma-separated string. Each token must be a known extension.
fn parse_lang_arg(args: Option<&Value>) -> Result<Vec<crate::lang::Language>, String> {
    let Some(value) = args.and_then(|a| a.get("lang")) else {
        return Ok(Vec::new());
    };
    let tokens: Vec<String> = match value {
        Value::String(s) => s
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.to_string())
            .collect(),
        _ => return Err("\"lang\" must be a string or an array of strings".to_string()),
    };
    let mut langs = Vec::new();
    for token in tokens {
        match crate::lang::Language::from_extension(&token) {
            Some(lang) => langs.push(lang),
            None => return Err(format!("unknown lang value {token:?}")),
        }
    }
    Ok(langs)
}

/// Parse the optional `focus` argument: an array of path strings, or a single
/// string. Unknown shapes yield an empty focus (unfocused rank).
fn parse_focus_arg(args: Option<&Value>) -> Vec<String> {
    match args.and_then(|a| a.get("focus")) {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Resolve `requested` (absolute, or relative to `root`) and confirm it is a
/// directory *inside* `root` — rejecting traversal outside the configured root
/// (#102). Symlinks are resolved (canonicalize) before the containment check, so
/// a symlink pointing outside the root is also rejected.
fn confine(requested: &str, root: &Path) -> Result<PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("server root {} is unavailable: {e}", root.display()))?;
    let raw = Path::new(requested);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        canonical_root.join(raw)
    };
    let target = joined
        .canonicalize()
        .map_err(|e| format!("cannot open {requested}: {e}"))?;
    if !target.starts_with(&canonical_root) {
        return Err(format!(
            "path {requested:?} is outside the allowed root {}",
            canonical_root.display()
        ));
    }
    if !target.is_dir() {
        return Err(format!("{requested} is not a directory"));
    }
    Ok(target)
}

/// Build the map via the supported library API (#69), read-only (#102): `cache:
/// false` so an agent's map pull never writes a `.atlas` cache into the target
/// repo.
fn build_map(
    root: &Path,
    budget: usize,
    no_private: bool,
    langs: Vec<crate::lang::Language>,
    focus: Vec<String>,
) -> Result<BudgetedMap, String> {
    crate::api::build_map(
        root,
        &crate::api::MapOptions {
            budget,
            no_private,
            langs,
            focus,
            cache: false,
        },
    )
    .map_err(|e| e.to_string())
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

    // A confinement root for the protocol-only tests (path is irrelevant there).
    fn any_root() -> &'static Path {
        Path::new(".")
    }

    #[test]
    fn initialize_advertises_tools_and_server() {
        let resp = handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}),
            any_root(),
        )
        .unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(resp["result"]["serverInfo"]["name"], "atlas");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_has_map_symbol_index_and_expand_tools() {
        let resp = handle(
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            any_root(),
        )
        .unwrap();
        assert_eq!(resp["result"]["tools"][0]["name"], "get_map");
        assert_eq!(
            resp["result"]["tools"][0]["inputSchema"]["required"][0],
            "path"
        );
        assert_eq!(resp["result"]["tools"][1]["name"], "get_symbol");
        let required = &resp["result"]["tools"][1]["inputSchema"]["required"];
        assert_eq!(required[0], "path");
        assert_eq!(required[1], "name");
        assert_eq!(resp["result"]["tools"][2]["name"], "get_symbol_index");
        assert_eq!(
            resp["result"]["tools"][2]["inputSchema"]["required"][0],
            "path"
        );
        assert_eq!(resp["result"]["tools"][3]["name"], "expand_symbol");
    }

    #[test]
    fn ping_returns_empty_result() {
        let resp = handle(&json!({"jsonrpc":"2.0","id":3,"method":"ping"}), any_root()).unwrap();
        assert!(resp["result"].is_object());
        assert!(resp.get("error").is_none());
    }

    #[test]
    fn notification_gets_no_response() {
        // No `id` → a notification → no reply.
        let req = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(handle(&req, any_root()).is_none());
    }

    #[test]
    fn unknown_method_is_jsonrpc_error() {
        let req = json!({"jsonrpc":"2.0","id":4,"method":"does/not/exist"});
        let resp = handle(&req, any_root()).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn get_map_missing_path_is_tool_error() {
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":5,"method":"tools/call",
                "params": { "name": "get_map", "arguments": {} }
            }),
            any_root(),
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn get_map_unknown_tool_is_tool_error() {
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":6,"method":"tools/call",
                "params": { "name": "nope", "arguments": {} }
            }),
            any_root(),
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn get_map_renders_a_real_tree() {
        let repo = temp_source_repo("map");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":7,"method":"tools/call",
                "params": { "name": "get_map", "arguments": { "path": path, "budget": 1024 } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("# atlas:"), "{text}");
        // Read-only (#102): mapping must not write a .atlas cache into the repo.
        assert!(!repo.join(".atlas").exists(), "MCP get_map wrote a cache");
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn get_map_json_format() {
        let repo = temp_source_repo("json");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":8,"method":"tools/call",
                "params": { "name": "get_map", "arguments": { "path": path, "format": "json" } }
            }),
            &repo,
        )
        .unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"version\":"), "{text}");
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn get_symbol_locates_a_definition() {
        let repo = temp_source_repo("sym");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":10,"method":"tools/call",
                "params": { "name": "get_symbol", "arguments": { "path": path, "name": "run" } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).expect("get_symbol returns JSON");
        assert_eq!(payload["name"], "run");
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["definitions"][0]["file"], "app.py");
        assert_eq!(payload["definitions"][0]["kind"], "function");
        assert!(payload["definitions"][0]["line"].as_u64().unwrap() >= 1);
        // Read-only (#102): get_symbol must not write a cache.
        assert!(!repo.join(".atlas").exists(), "get_symbol wrote a cache");
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn get_symbol_unknown_name_is_empty_not_error() {
        let repo = temp_source_repo("sym-missing");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":11,"method":"tools/call",
                "params": { "name": "get_symbol", "arguments": { "path": path, "name": "nope" } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).unwrap();
        assert_eq!(payload["count"], 0);
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn get_symbol_missing_name_is_tool_error() {
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":12,"method":"tools/call",
                "params": { "name": "get_symbol", "arguments": { "path": "." } }
            }),
            any_root(),
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn get_symbol_index_returns_anchors_without_signatures() {
        let repo = temp_source_repo("index");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":20,"method":"tools/call",
                "params": { "name": "get_symbol_index", "arguments": { "path": path, "budget": 512 } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).expect("get_symbol_index returns JSON");
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["entries"][0]["anchor"], "app.py#run");
        assert_eq!(payload["entries"][0]["name"], "run");
        assert_eq!(payload["entries"][0]["file"], "app.py");
        assert_eq!(payload["entries"][0]["kind"], "function");
        assert!(payload["entries"][0].get("signature").is_none());
        // Read-only (#102): indexing must not write a cache.
        assert!(
            !repo.join(".atlas").exists(),
            "get_symbol_index wrote a cache"
        );
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn expand_symbol_returns_signature_and_neighbors() {
        let repo = temp_source_repo("expand");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":21,"method":"tools/call",
                "params": { "name": "expand_symbol", "arguments": { "path": path, "anchor": "app.py#run" } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).expect("expand_symbol returns JSON");
        assert_eq!(payload["found"], true);
        assert_eq!(payload["name"], "run");
        assert_eq!(payload["file"], "app.py");
        assert_eq!(payload["kind"], "function");
        assert!(payload["signature"].as_str().unwrap().contains("run"));
        assert!(payload["imports"].is_array());
        assert!(payload["used_by"].is_array());
        // Read-only (#102): expand_symbol must not write a cache.
        assert!(!repo.join(".atlas").exists(), "expand_symbol wrote a cache");
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn expand_symbol_unknown_anchor_is_found_false() {
        let repo = temp_source_repo("expand-miss");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":22,"method":"tools/call",
                "params": { "name": "expand_symbol", "arguments": { "path": path, "anchor": "app.py#nope" } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).unwrap();
        assert_eq!(payload["found"], false);
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn expand_symbol_malformed_anchor_is_tool_error() {
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":23,"method":"tools/call",
                "params": { "name": "expand_symbol", "arguments": { "path": ".", "anchor": "noHashHere" } }
            }),
            any_root(),
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn parse_anchor_splits_file_name_and_optional_line() {
        assert_eq!(
            parse_anchor("src/x.rs#Foo"),
            Some(("src/x.rs", "Foo", None))
        );
        assert_eq!(
            parse_anchor("src/x.rs#from@42"),
            Some(("src/x.rs", "from", Some(42)))
        );
        // A non-numeric @suffix is part of the name, not a line pin.
        assert_eq!(
            parse_anchor("src/x.rs#Foo@bar"),
            Some(("src/x.rs", "Foo@bar", None))
        );
        assert_eq!(parse_anchor("nohash"), None);
        assert_eq!(parse_anchor("a#"), None);
        assert_eq!(parse_anchor("#name"), None);
    }

    #[test]
    fn get_map_unknown_lang_is_tool_error() {
        let repo = temp_source_repo("badlang");
        let path = repo.to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":13,"method":"tools/call",
                "params": { "name": "get_map", "arguments": { "path": path, "lang": ["cobol"] } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown lang"), "{text}");
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn get_map_outside_root_is_rejected() {
        // Confine to the repo, then request its parent — a traversal outside the
        // allowed root (#102) → tool error, not a map.
        let repo = temp_source_repo("escape");
        let parent = repo.parent().unwrap().to_string_lossy().to_string();
        let resp = handle(
            &json!({
                "jsonrpc":"2.0","id":9,"method":"tools/call",
                "params": { "name": "get_map", "arguments": { "path": parent } }
            }),
            &repo,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("outside the allowed root"), "{text}");
        let _ = fs::remove_dir_all(repo);
    }
}
