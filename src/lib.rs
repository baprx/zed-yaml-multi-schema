//! Zed extension entrypoint for per-block YAML JSON Schema validation.

pub mod completion;
pub mod document;
pub mod resolver;
pub mod server;
pub mod validator;

use zed_extension_api::{self as zed, Command, Extension, LanguageServerId, Result, Worktree};

/// The extension provides a YAML language server.
struct YamlMultiSchemaExtension;

impl Extension for YamlMultiSchemaExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        // This extension ships a language-server binary (the same package
        // compiled as a bin target) that speaks LSP over stdio. Locate it on
        // the PATH and launch it.
        let binary_path = worktree
            .which("zed-yaml-multi-schema-lsp")
            .ok_or_else(|| "zed-yaml-multi-schema-lsp not found in PATH".to_string())?;
        Ok(Command::new(binary_path))
    }
}

zed::register_extension!(YamlMultiSchemaExtension);
