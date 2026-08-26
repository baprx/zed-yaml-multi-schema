//! Language Server Protocol server over stdio.
//!
//! This binary is launched by the Zed extension (`language_server_command`) and
//! speaks a minimal subset of LSP: initialization, text document sync, scoped
//! diagnostics, and completion.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::{json, Value};
use zed_yaml_multi_schema::resolver::{ResolveKind, SchemaFetcher};
use zed_yaml_multi_schema::server::YamlServer;

/// Fetcher that reads local files from disk and remote HTTPS URLs via `ureq`.
struct FsFetcher;

impl SchemaFetcher for FsFetcher {
    fn read_local(&self, path: &str) -> Result<String, String> {
        let p = Path::new(path);
        std::fs::read_to_string(p).map_err(|e| e.to_string())
    }

    fn fetch_remote(&self, url: &str) -> Result<String, String> {
        if classify(url) != ResolveKind::Remote {
            return Err("not a remote URL".to_string());
        }
        let body = ureq::get(url)
            .call()
            .map_err(|e| format!("request failed: {e}"))?
            .into_string()
            .map_err(|e| format!("read body failed: {e}"))?;
        Ok(body)
    }
}

fn classify(reference: &str) -> ResolveKind {
    if let Ok(u) = url::Url::parse(reference) {
        if u.scheme() == "https" {
            return ResolveKind::Remote;
        }
    }
    ResolveKind::Local
}

/// Reads one Content-Length framed JSON-RPC message from `input`.
fn read_message(input: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = input.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let len = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0u8; len];
    input.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut out = stdout.lock();

    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let fetcher = FsFetcher;
    let mut server = YamlServer::new(&fetcher, &root);
    let mut documents: HashMap<String, String> = HashMap::new();

    eprintln!(
        "zed-yaml-multi-schema-lsp: started (worktree root: {})",
        root.display()
    );

    while let Some(raw) = read_message(&mut reader)? {
        let msg: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let method = msg.get("method").and_then(|m| m.as_str());
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        match method {
            Some("initialize") => {
                eprintln!("zed-yaml-multi-schema-lsp: initialize");
                let resp = json!({
                    "capabilities": {
                        "textDocumentSync": 2,
                        "completionProvider": {"resolveProvider": false}
                    },
                    "serverInfo": {"name": "zed-yaml-multi-schema", "version": "0.1.0"}
                });
                send(&mut out, id, Some(resp), None)?;
            }
            Some("initialized") | Some("textDocument/didClose") => {}
            Some("textDocument/didOpen") | Some("textDocument/didChange") => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default();
                let text = params
                    .pointer("/textDocument/text")
                    .and_then(|t| t.as_str());
                if let Some(text) = text {
                    documents.insert(uri.to_string(), text.to_string());
                    server.on_change(text);
                    eprintln!(
                        "zed-yaml-multi-schema-lsp: {} {} ({}) -> {} diagnostic(s)",
                        method.unwrap_or("?"),
                        uri,
                        text.lines().count(),
                        server.diagnostics().len()
                    );
                    publish_diagnostics(&mut out, uri, &server)?;
                }
            }
            Some("textDocument/completion") => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default();
                let line = params
                    .pointer("/position/line")
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0) as usize;
                let completions = server.complete_at_line(line);
                eprintln!(
                    "zed-yaml-multi-schema-lsp: completion at line {} of {} -> {} item(s)",
                    line,
                    uri,
                    completions.len()
                );
                let items: Vec<Value> = completions
                    .into_iter()
                    .map(|c| json!({"label": c.label, "kind": c.kind, "detail": c.detail}))
                    .collect();
                send(
                    &mut out,
                    id,
                    Some(json!({"isIncomplete": false, "items": items})),
                    None,
                )?;
                let _ = uri;
            }
            Some("shutdown") => {
                send(&mut out, id, Some(Value::Null), None)?;
            }
            Some("exit") => break,
            _ => {
                if id.is_some() {
                    send(
                        &mut out,
                        id,
                        None,
                        Some(json!({"code": -32601, "message": "method not found"})),
                    )?;
                }
            }
        }
        out.flush()?;
    }
    Ok(())
}

fn publish_diagnostics(out: &mut impl Write, uri: &str, server: &YamlServer) -> io::Result<()> {
    let diags: Vec<Value> = server
        .diagnostics()
        .iter()
        .map(|d| {
            json!({
                "range": {
                    "start": {"line": d.start_line, "character": 0},
                    "end": {"line": d.end_line, "character": 0}
                },
                "severity": 1,
                "message": d.message,
                "source": "zed-yaml-multi-schema"
            })
        })
        .collect();
    let params = json!({"uri": uri, "diagnostics": diags});
    send(out, None, Some(params), None)?;
    Ok(())
}

fn send(
    out: &mut impl Write,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<Value>,
) -> io::Result<()> {
    let mut body = json!({});
    body["jsonrpc"] = json!("2.0");
    if let Some(id) = id {
        body["id"] = id;
    }
    if let Some(result) = result {
        body["result"] = result;
    }
    if let Some(error) = error {
        body["error"] = error;
    }
    if body.get("id").is_none() && body.get("result").is_none() && body.get("error").is_none() {
        // Must be a notification; nothing to send.
        return Ok(());
    }
    let payload = serde_json::to_string(&body)?;
    write!(out, "Content-Length: {}\r\n\r\n{}", payload.len(), payload)?;
    Ok(())
}
