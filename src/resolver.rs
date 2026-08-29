//! Schema resolution: load schemas from local relative paths and remote HTTPS
//! URLs, with caching and graceful failure.

use jsonschema::Validator;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// How a schema reference should be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveKind {
    /// Local file path (relative or absolute).
    Local,
    /// Remote HTTPS URL.
    Remote,
}

/// Abstract fetch capability, so the core logic is testable without the WASM
/// host. The extension wires this to `zed::fs` / `zed::http_client`.
pub trait SchemaFetcher: Send + Sync {
    fn read_local(&self, path: &str) -> Result<String, String>;
    fn fetch_remote(&self, url: &str) -> Result<String, String>;
}

/// Outcome of resolving a schema reference.
#[derive(Debug, Clone)]
pub enum ResolveOutcome {
    /// Successfully resolved and parsed a JSON schema.
    Resolved {
        /// The parsed schema value.
        schema: serde_json::Value,
    },
    /// Resolution failed; the reference cannot be used.
    Failed {
        /// Human-readable reason for the failure.
        reason: String,
    },
}

/// Determines how a reference should be resolved.
pub fn classify(reference: &str) -> ResolveKind {
    if let Ok(url) = url::Url::parse(reference) {
        if url.scheme() == "https" {
            return ResolveKind::Remote;
        }
    }
    ResolveKind::Local
}

/// Lexically normalizes a path, removing `.` components.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolves and caches schemas keyed by reference string.
pub struct SchemaResolver<'a> {
    fetcher: Arc<dyn SchemaFetcher>,
    /// Worktree root used to resolve relative local paths.
    worktree_root: &'a Path,
    cache: HashMap<String, ResolveOutcome>,
    validator_cache: HashMap<String, Arc<Validator>>,
}

impl<'a> SchemaResolver<'a> {
    pub fn new(fetcher: Arc<dyn SchemaFetcher>, worktree_root: &'a Path) -> Self {
        Self {
            fetcher,
            worktree_root,
            cache: HashMap::new(),
            validator_cache: HashMap::new(),
        }
    }

    pub fn fetcher(&self) -> Arc<dyn SchemaFetcher> {
        Arc::clone(&self.fetcher)
    }

    /// Resolves `reference`, using the cache when available.
    ///
    /// Successful resolutions are cached; failures are not, so a reference that
    /// becomes resolvable later (e.g. after a transient network error) is
    /// re-attempted on the next call.
    pub fn resolve(&mut self, reference: &str) -> ResolveOutcome {
        if let Some(cached) = self.cache.get(reference) {
            return cached.clone();
        }

        let outcome = self.resolve_uncached(reference);
        if let ResolveOutcome::Resolved { .. } = &outcome {
            self.cache.insert(reference.to_string(), outcome.clone());
        }
        outcome
    }

    /// Returns a compiled Validator for `reference`, building (and caching)
    /// it only the first time this reference is seen. `schema` is the
    /// already-resolved root schema document (from `self.resolve`).
    pub fn validator_for(
        &mut self,
        reference: &str,
        schema: &serde_json::Value,
    ) -> Result<Arc<Validator>, String> {
        if let Some(v) = self.validator_cache.get(reference) {
            return Ok(Arc::clone(v));
        }

        let validator = Validator::options()
            .with_retriever(crate::validator::FetcherRetriever {
                fetcher: Arc::clone(&self.fetcher),
            })
            .build(schema)
            .map_err(|e| format!("invalid schema: {e}"))?;

        let validator = Arc::new(validator);
        self.validator_cache
            .insert(reference.to_string(), Arc::clone(&validator));
        Ok(validator)
    }

    fn resolve_uncached(&self, reference: &str) -> ResolveOutcome {
        let raw = match classify(reference) {
            ResolveKind::Local => {
                let joined = if Path::new(reference).is_absolute() {
                    PathBuf::from(reference)
                } else {
                    self.worktree_root.join(reference)
                };
                let path = normalize(&joined).to_string_lossy().to_string();
                match self.fetcher.read_local(&path) {
                    Ok(text) => text,
                    Err(e) => {
                        return ResolveOutcome::Failed {
                            reason: format!("failed to read local schema '{reference}': {e}"),
                        }
                    }
                }
            }
            ResolveKind::Remote => match self.fetcher.fetch_remote(reference) {
                Ok(text) => text,
                Err(e) => {
                    return ResolveOutcome::Failed {
                        reason: format!("failed to fetch remote schema '{reference}': {e}"),
                    }
                }
            },
        };

        match serde_json::from_str(&raw) {
            Ok(schema) => ResolveOutcome::Resolved { schema },
            Err(e) => ResolveOutcome::Failed {
                reason: format!("invalid JSON schema '{reference}': {e}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        local: HashMap<String, String>,
        remote: HashMap<String, String>,
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

    fn schema_json() -> String {
        r#"{"type":"object","properties":{"enabled":{"type":"boolean"}}}"#.to_string()
    }

    #[test]
    fn resolves_local_relative_to_root() {
        let mut local = HashMap::new();
        local.insert("/root/schemas/test.schema.json".to_string(), schema_json());
        let fetcher = FakeFetcher {
            local,
            remote: HashMap::new(),
        };
        let mut resolver = SchemaResolver::new(Arc::new(fetcher), Path::new("/root"));

        match resolver.resolve("./schemas/test.schema.json") {
            ResolveOutcome::Resolved { schema } => {
                assert_eq!(schema["type"], "object");
            }
            ResolveOutcome::Failed { reason } => panic!("unexpected failure: {reason}"),
        }
    }

    #[test]
    fn resolves_remote_https() {
        let mut remote = HashMap::new();
        remote.insert(
            "https://example.com/v.schema.json".to_string(),
            schema_json(),
        );
        let fetcher = FakeFetcher {
            local: HashMap::new(),
            remote,
        };
        let mut resolver = SchemaResolver::new(Arc::new(fetcher), Path::new("/root"));
        assert!(matches!(
            resolver.resolve("https://example.com/v.schema.json"),
            ResolveOutcome::Resolved { .. }
        ));
    }

    #[test]
    fn fails_gracefully_on_missing() {
        let fetcher = FakeFetcher {
            local: HashMap::new(),
            remote: HashMap::new(),
        };
        let mut resolver = SchemaResolver::new(Arc::new(fetcher), Path::new("/root"));
        match resolver.resolve("./missing.schema.json") {
            ResolveOutcome::Resolved { .. } => panic!("expected failure"),
            ResolveOutcome::Failed { reason } => assert!(reason.contains("missing.schema.json")),
        }
    }
}
