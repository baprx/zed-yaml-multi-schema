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
        "yaml-multi-schema-lsp: started (worktree root: {})",
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
                eprintln!("yaml-multi-schema-lsp: initialize");
                send(&mut out, id, Some(initialize_result()), None)?;
            }
            Some("initialized") | Some("textDocument/didClose") => {}
            Some("textDocument/didOpen") | Some("textDocument/didChange") => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default();
                // `didOpen` carries the text in `/textDocument/text`; `didChange`
                // (full sync) carries it in `/contentChanges/0/text`.
                let text = document_text(&params);
                if let Some(text) = text {
                    documents.insert(uri.to_string(), text.clone());
                    server.on_change(&text);
                    eprintln!(
                        "yaml-multi-schema-lsp: {} {} ({}) -> {} diagnostic(s)",
                        method.unwrap_or("?"),
                        uri,
                        text.lines().count(),
                        server.diagnostics().len()
                    );
                    publish_diagnostics(&mut out, uri, &server, &text)?;
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
                let character = params
                    .pointer("/position/character")
                    .and_then(|c| c.as_u64())
                    .unwrap_or(0) as usize;
                let text = documents.get(uri).cloned().unwrap_or_default();
                let completions = server.complete_at(&text, line, character);
                eprintln!(
                    "yaml-multi-schema-lsp: completion at {}:{} of {} -> {} item(s)",
                    line,
                    character,
                    uri,
                    completions.len()
                );
                let items: Vec<Value> = completions
                    .into_iter()
                    .map(|c| {
                        let mut item =
                            json!({"label": c.label, "kind": c.kind, "detail": c.detail});
                        if let Some(text) = c.insert_text {
                            item["insertText"] = json!(text);
                            if let Some(fmt) = c.insert_text_format {
                                item["insertTextFormat"] = json!(fmt);
                            }
                        }
                        item
                    })
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

fn initialize_result() -> Value {
    let mut trigger_chars: Vec<String> = (b'a'..=b'z')
        .chain(b'A'..=b'Z')
        .map(|c| char::from(c).to_string())
        .collect();
    // Declaring letters as trigger characters makes Zed open the completion menu
    // as soon as a key name is typed on a fresh line, not only after ':'/'-'.
    trigger_chars.extend([":".to_string(), "-".to_string()]);
    json!({
        "capabilities": {
            "textDocumentSync": 1,
            "completionProvider": {
                "resolveProvider": false,
                "triggerCharacters": trigger_chars
            }
        },
        "serverInfo": {"name": "yaml-multi-schema", "version": env!("CARGO_PKG_VERSION")}
    })
}

fn publish_diagnostics(
    out: &mut impl Write,
    uri: &str,
    server: &YamlServer,
    text: &str,
) -> io::Result<()> {
    let lines: Vec<&str> = text.lines().collect();
    let line_len = |line: usize| lines.get(line).map(|l| l.chars().count()).unwrap_or(0);
    let diags: Vec<Value> = server
        .diagnostics()
        .iter()
        .map(|d| {
            json!({
                "range": {
                    "start": {"line": d.start_line, "character": 0},
                    "end": {"line": d.end_line, "character": line_len(d.end_line)}
                },
                "severity": lsp_severity(&d.severity),
                "message": d.message,
                "source": "yaml-multi-schema"
            })
        })
        .collect();
    let params = json!({"uri": uri, "diagnostics": diags});
    send_notification(out, "textDocument/publishDiagnostics", params)?;
    Ok(())
}

/// Maps our string severity to the LSP DiagnosticSeverity integer:
/// Error=1, Warning=2, Information=3, Hint=4.
fn lsp_severity(severity: &str) -> i64 {
    match severity {
        "warning" => 2,
        "info" => 3,
        "hint" => 4,
        _ => 1,
    }
}

fn send_notification(out: &mut impl Write, method: &str, params: Value) -> io::Result<()> {
    let body = json!({"jsonrpc": "2.0", "method": method, "params": params});
    let payload = serde_json::to_string(&body)?;
    write!(out, "Content-Length: {}\r\n\r\n{}", payload.len(), payload)?;
    Ok(())
}

/// Extracts the full document text from a `didOpen` or `didChange` (full sync)
/// request: `didOpen` uses `/textDocument/text`, `didChange` uses
/// `/contentChanges/0/text`.
fn document_text(params: &Value) -> Option<String> {
    params
        .pointer("/textDocument/text")
        .and_then(|t| t.as_str())
        .map(String::from)
        .or_else(|| {
            params
                .pointer("/contentChanges/0/text")
                .and_then(|t| t.as_str())
                .map(String::from)
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_text_from_did_open() {
        let params = json!({
            "textDocument": {"uri": "file:///a.yaml", "text": "a: 1\n"}
        });
        assert_eq!(document_text(&params).as_deref(), Some("a: 1\n"));
    }

    #[test]
    fn document_text_from_did_change_full_sync() {
        let params = json!({
            "textDocument": {"uri": "file:///a.yaml"},
            "contentChanges": [{"text": "a: 2\n"}]
        });
        assert_eq!(document_text(&params).as_deref(), Some("a: 2\n"));
    }

    #[test]
    fn document_text_absent_when_no_text() {
        let params = json!({"textDocument": {"uri": "file:///a.yaml"}});
        assert_eq!(document_text(&params), None);
    }

    #[test]
    fn initialize_declares_completion_trigger_characters() {
        let resp = initialize_result();
        let chars = resp["capabilities"]["completionProvider"]["triggerCharacters"]
            .as_array()
            .unwrap();
        assert!(chars.iter().any(|c| c == ":"), "should trigger on ':'");
        assert!(chars.iter().any(|c| c == "-"), "should trigger on '-'");
        assert!(
            chars.iter().any(|c| c == "i"),
            "should trigger on letters so key names auto-complete while typing"
        );
    }
}
