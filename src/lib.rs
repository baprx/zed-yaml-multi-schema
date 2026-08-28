//! Zed extension entrypoint for per-block YAML JSON Schema validation.

pub mod completion;
pub mod document;
pub mod install;
pub mod resolver;
pub mod server;
pub mod validator;

use zed_extension_api::{self as zed, Command, Extension, LanguageServerId, Result, Worktree};

use crate::install::BINARY_NAME;

/// The extension provides a YAML language server.
struct YamlMultiSchemaExtension;

impl Extension for YamlMultiSchemaExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        // 1. Dev extension flow: a locally built binary on the PATH wins and
        //    nothing is downloaded.
        if let Some(binary_path) = worktree.which(BINARY_NAME) {
            return Ok(Command {
                command: binary_path,
                args: vec![],
                env: Default::default(),
            });
        }

        // 2. Published flow: resolve the platform, then reuse the cached
        //    download for this extension version or fetch it from the
        //    matching GitHub release.
        let (os, arch) = zed::current_platform();
        let triple = install::platform_triple(os, arch)?;
        let version = env!("CARGO_PKG_VERSION");
        let binary_path = install::binary_path(version);
        if !std::fs::metadata(&binary_path).is_ok_and(|metadata| metadata.is_file()) {
            install::download_language_server(language_server_id, triple, version)?;
        }
        Ok(Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(YamlMultiSchemaExtension);
