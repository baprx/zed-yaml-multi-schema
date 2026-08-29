//! Shared helpers for integration tests.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;

use std::sync::Arc;
use zed_yaml_multi_schema::resolver::{ResolveKind, SchemaFetcher};
use zed_yaml_multi_schema::server::YamlServer;

/// Fake fetcher serving local paths from an in-memory map and remote URLs from
/// another in-memory map, so tests never touch the network.
pub struct FakeFetcher {
    pub local: HashMap<String, String>,
    pub remote: HashMap<String, String>,
}

impl FakeFetcher {
    pub fn new() -> Self {
        Self {
            local: HashMap::new(),
            remote: HashMap::new(),
        }
    }

    pub fn add_local(&mut self, path: &str, content: &str) {
        self.local.insert(path.to_string(), content.to_string());
    }

    pub fn add_remote(&mut self, url: &str, content: &str) {
        self.remote.insert(url.to_string(), content.to_string());
    }

    pub fn server(self, root: &Path) -> YamlServer<'_> {
        YamlServer::new(Arc::new(self), root)
    }
}

impl SchemaFetcher for FakeFetcher {
    fn read_local(&self, path: &str) -> Result<String, String> {
        self.local
            .get(path)
            .cloned()
            .ok_or_else(|| format!("no local file '{path}'"))
    }

    fn fetch_remote(&self, url: &str) -> Result<String, String> {
        self.remote
            .get(url)
            .cloned()
            .ok_or_else(|| format!("no remote '{url}'"))
    }
}

pub const LOCAL_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "enabled": {"type": "boolean", "description": "Whether enabled"},
    "elements": {"type": "array", "items": {"type": ["string", "number"]}},
    "product": {"type": "string"},
    "version": {"type": "number"}
  },
  "required": ["enabled", "elements", "product", "version"]
}"#;

pub const REMOTE_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "image": {
      "type": "object",
      "properties": {
        "registry": {"type": "string"},
        "repository": {"type": "string"},
        "tag": {"type": "string"}
      }
    },
    "enabled": {"type": "boolean"}
  }
}"#;

/// Classifies a reference for constructing fixture paths.
pub fn kind(reference: &str) -> ResolveKind {
    if let Ok(u) = url::Url::parse(reference) {
        if u.scheme() == "https" {
            return ResolveKind::Remote;
        }
    }
    ResolveKind::Local
}
