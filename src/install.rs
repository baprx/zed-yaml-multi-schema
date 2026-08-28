//! Language-server installation helpers.
//!
//! Pure logic (platform mapping, asset naming, install directory derivation)
//! plus the download orchestration used by `language_server_command`, and the
//! regression guard for the cdylib output name that Zed's extension builder
//! expects.

use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, LanguageServerId,
    LanguageServerInstallationStatus, Os,
};

/// Name of the language-server binary, both inside release archives and in
/// versioned install directories.
pub const BINARY_NAME: &str = "yaml-multi-schema-lsp";

/// GitHub repository (`owner/name`) that publishes the language-server
/// release assets.
pub const GITHUB_REPO: &str = "baprx/zed-yaml-multi-schema";

/// Maps the runtime platform reported by Zed to the Rust target triple used
/// in release asset names. Unsupported combinations yield a descriptive
/// error rather than a guessed triple.
pub fn platform_triple(os: Os, arch: Architecture) -> Result<&'static str, String> {
    match (os, arch) {
        (Os::Mac, Architecture::Aarch64) => Ok("aarch64-apple-darwin"),
        (Os::Mac, Architecture::X8664) => Ok("x86_64-apple-darwin"),
        (Os::Linux, Architecture::X8664) => Ok("x86_64-unknown-linux-gnu"),
        (os, arch) => Err(format!(
            "unsupported platform: {os:?} on {arch:?}; this extension provides \
             language-server binaries for macOS (arm64, x86_64) and Linux (x86_64) only"
        )),
    }
}

/// Release asset file name for the given target triple, as produced by the
/// release CI workflow (see specs/002-publishing-readiness/contracts/release-assets.md).
pub fn asset_name(triple: &str) -> String {
    format!("{BINARY_NAME}-{triple}.tar.gz")
}

/// Versioned install directory (relative to the extension working directory)
/// into which the release archive is extracted.
pub fn install_dir(version: &str) -> String {
    format!("{BINARY_NAME}-v{version}")
}

/// Relative path of the language-server binary inside the extension working
/// directory for the given extension version. Deterministic because the
/// release is looked up by exact tag, so callers can check for an existing
/// download without any network I/O.
pub fn binary_path(version: &str) -> String {
    format!("{}/{BINARY_NAME}", install_dir(version))
}

/// Downloads the language-server binary for `triple` from the GitHub release
/// tagged `v{version}`, makes it executable, cleans up stale versions, and
/// returns the relative binary path.
///
/// Errors always name the version and the asset/platform so a missing
/// release or asset (e.g. the extensions-registry PR being merged before the
/// release CI ran) is immediately diagnosable.
pub fn download_language_server(
    language_server_id: &LanguageServerId,
    triple: &str,
    version: &str,
) -> Result<String, String> {
    zed::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::CheckingForUpdate,
    );
    let release = zed::github_release_by_tag_name(GITHUB_REPO, &format!("v{version}"))
        .map_err(|error| format!("release v{version} of {GITHUB_REPO} not found: {error}"))?;

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name(triple))
        .ok_or_else(|| {
            format!(
                "no asset {} in release v{version} of {GITHUB_REPO}; the release \
                 CI workflow may not have uploaded platform binaries yet",
                asset_name(triple)
            )
        })?;

    let dir = install_dir(version);
    zed::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::Downloading,
    );
    zed::download_file(&asset.download_url, &dir, DownloadedFileType::GzipTar)
        .map_err(|error| format!("failed to download {} (v{version}): {error}", asset.name))?;

    let path = binary_path(version);
    zed::make_file_executable(&path)
        .map_err(|error| format!("failed to make {path} executable: {error}"))?;

    cleanup_stale_versions(&dir);
    Ok(path)
}

/// Removes versioned install directories other than the current one. Best
/// effort: cleanup failures never fail the install.
fn cleanup_stale_versions(current_dir: &str) {
    let entries = match std::fs::read_dir(".") {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(BINARY_NAME) && name != current_dir {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Returns `true` when the `[lib]` table of the manifest overrides the
/// library name. Zed derives the expected wasm file name from
/// `[package] name` only (`-` → `_`, extension `.wasm`), so an override
/// breaks the dev-extension build.
#[cfg(test)]
fn has_lib_name_override(manifest: &str) -> bool {
    let mut in_lib = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_lib = trimmed == "[lib]";
            continue;
        }
        if in_lib {
            if let Some(key) = trimmed.split('=').next() {
                if key.trim() == "name" {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_triple_maps_supported_platforms() {
        assert_eq!(
            platform_triple(Os::Mac, Architecture::Aarch64).unwrap(),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            platform_triple(Os::Mac, Architecture::X8664).unwrap(),
            "x86_64-apple-darwin"
        );
        assert_eq!(
            platform_triple(Os::Linux, Architecture::X8664).unwrap(),
            "x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn platform_triple_errors_on_unsupported_platforms() {
        for (os, arch) in [
            (Os::Windows, Architecture::X8664),
            (Os::Linux, Architecture::Aarch64),
            (Os::Linux, Architecture::X86),
        ] {
            let err = platform_triple(os, arch).unwrap_err();
            assert!(err.contains("unsupported platform"), "message: {err}");
            assert!(err.contains(&format!("{os:?}")), "message: {err}");
            assert!(err.contains(&format!("{arch:?}")), "message: {err}");
        }
    }

    #[test]
    fn asset_name_matches_release_ci_contract() {
        assert_eq!(
            asset_name("x86_64-unknown-linux-gnu"),
            "yaml-multi-schema-lsp-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            asset_name("aarch64-apple-darwin"),
            "yaml-multi-schema-lsp-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn install_dir_is_versioned() {
        assert_eq!(install_dir("0.5.0"), "yaml-multi-schema-lsp-v0.5.0");
    }

    #[test]
    fn cdylib_output_name_matches_zed_expectation() {
        let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .expect("Cargo.toml must be readable");
        assert!(
            !has_lib_name_override(&manifest),
            "[lib] name override present: Zed derives the expected wasm file name \
             from [package] name only (crate/extension/src/extension_builder.rs), \
             so an override makes `Install Dev Extension` fail to find \
             target/wasm32-wasip2/<profile>/<lib>.wasm"
        );
        let package_name = manifest
            .lines()
            .skip_while(|line| !line.trim().starts_with("[package]"))
            .skip(1)
            .take_while(|line| !line.trim().starts_with('['))
            .find_map(|line| {
                let mut parts = line.splitn(2, '=');
                let key = parts.next()?.trim();
                if key != "name" {
                    return None;
                }
                parts
                    .next()
                    .map(|value| value.trim().trim_matches('"').to_string())
            })
            .expect("[package] name must be present");
        let wasm = format!("{}.wasm", package_name.replace('-', "_"));
        assert_eq!(
            wasm, "zed_yaml_multi_schema.wasm",
            "Zed reads target/wasm32-wasip2/<profile>/{wasm}; if this changes, \
             dev extension installs break again"
        );
    }
}
