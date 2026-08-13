//! Installs the extensions bundled with Lexed into the user's extensions
//! directory, so they are pre-installed on first launch with no network access
//! or user action.
//!
//! The packaged extension artifacts (compiled `extension.wasm`, grammar wasm,
//! language configs) are vendored under this crate's `extensions/` directory —
//! one subdirectory per extension id — and embedded into the binary. They are
//! produced by `script/lexed-package-lex-extension` from a checkout of the
//! extension's own repository.
//!
//! Installation is a plain file copy into `<data>/extensions/installed/<id>/`,
//! the same layout the extension store uses for extensions downloaded from a
//! registry, so the store loads them through its normal scan with no changes
//! to upstream code.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "extensions"]
struct BundledExtensions;

/// Copies each bundled extension into the installed-extensions directory.
///
/// Must run before the extension store's initial scan so that a first launch
/// picks the extensions up immediately (later launches would also be covered
/// by the store's directory watcher). The copied payload is small, so doing
/// this synchronously on startup is fine.
pub fn init() {
    let installed_dir = paths::extensions_dir().join("installed");
    for extension_id in bundled_extension_ids() {
        if let Err(error) = install_extension(&installed_dir, &extension_id) {
            log::error!("failed to install bundled extension {extension_id}: {error:#}");
        }
    }
}

fn bundled_extension_ids() -> BTreeSet<String> {
    BundledExtensions::iter()
        .filter_map(|path| {
            let (extension_id, _) = path.split_once('/')?;
            Some(extension_id.to_string())
        })
        .collect()
}

fn install_extension(installed_dir: &Path, extension_id: &str) -> Result<()> {
    let destination = installed_dir.join(extension_id);

    // A symlink here is a locally-installed dev extension; never clobber it.
    if destination
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.is_symlink())
    {
        log::info!("bundled extension {extension_id}: dev extension symlink present, skipping");
        return Ok(());
    }

    let bundled_version = bundled_manifest_version(extension_id)?;
    if installed_manifest_version(&destination).as_deref() == Some(bundled_version.as_str()) {
        return Ok(());
    }

    // Stage next to the destination and rename, so the extension store's
    // directory watcher never observes a half-copied extension.
    let staging_dir = installed_dir.join(format!(".{extension_id}.lexed-staging"));
    if staging_dir.exists() {
        std::fs::remove_dir_all(&staging_dir)?;
    }

    let prefix = format!("{extension_id}/");
    for path in BundledExtensions::iter() {
        let Some(relative_path) = path.strip_prefix(&prefix) else {
            continue;
        };
        let file = BundledExtensions::get(&path)
            .with_context(|| format!("missing embedded file {path}"))?;
        let file_path = staging_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, file.data)?;
    }

    if destination.exists() {
        std::fs::remove_dir_all(&destination)?;
    }
    std::fs::rename(&staging_dir, &destination)?;
    log::info!("installed bundled extension {extension_id} {bundled_version}");
    Ok(())
}

#[derive(serde::Deserialize)]
struct ManifestVersion {
    version: String,
}

fn bundled_manifest_version(extension_id: &str) -> Result<String> {
    let manifest = BundledExtensions::get(&format!("{extension_id}/extension.toml"))
        .ok_or_else(|| anyhow!("bundled extension {extension_id} has no extension.toml"))?;
    let manifest = std::str::from_utf8(&manifest.data)?;
    let manifest: ManifestVersion =
        toml::from_str(manifest).context("failed to parse bundled extension.toml")?;
    Ok(manifest.version)
}

fn installed_manifest_version(extension_dir: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(extension_dir.join("extension.toml")).ok()?;
    let manifest: ManifestVersion = toml::from_str(&manifest).ok()?;
    Some(manifest.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_lex_extension_is_complete() {
        assert!(bundled_extension_ids().contains("lex"));
        for required in [
            "lex/extension.toml",
            "lex/extension.wasm",
            "lex/grammars/lex.wasm",
            "lex/languages/lex/config.toml",
            "lex/languages/lex/highlights.scm",
        ] {
            assert!(
                BundledExtensions::get(required).is_some(),
                "missing {required}; run script/lexed-package-lex-extension"
            );
        }
        let version = bundled_manifest_version("lex").expect("manifest version");
        assert!(!version.is_empty());
    }

    #[test]
    fn installs_into_empty_dir() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let installed_dir = temp_dir.path().join("installed");

        install_extension(&installed_dir, "lex").expect("install");

        let lex_dir = installed_dir.join("lex");
        assert!(lex_dir.join("extension.toml").is_file());
        assert!(lex_dir.join("extension.wasm").is_file());
        assert!(lex_dir.join("grammars/lex.wasm").is_file());
        assert!(lex_dir.join("languages/lex/config.toml").is_file());
        assert!(!installed_dir.join(".lex.lexed-staging").exists());
    }

    #[test]
    fn skips_reinstall_when_version_matches() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let installed_dir = temp_dir.path().join("installed");
        install_extension(&installed_dir, "lex").expect("install");

        // A marker file survives only if the second install is a no-op.
        let marker = installed_dir.join("lex/marker");
        std::fs::write(&marker, b"marker").expect("write marker");

        install_extension(&installed_dir, "lex").expect("reinstall");
        assert!(marker.is_file(), "same-version install must not rewrite");
    }

    #[test]
    fn reinstalls_when_version_differs() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let installed_dir = temp_dir.path().join("installed");
        install_extension(&installed_dir, "lex").expect("install");

        let manifest_path = installed_dir.join("lex/extension.toml");
        std::fs::write(&manifest_path, "id = \"lex\"\nversion = \"0.0.0\"\n")
            .expect("write stale manifest");
        let marker = installed_dir.join("lex/marker");
        std::fs::write(&marker, b"marker").expect("write marker");

        install_extension(&installed_dir, "lex").expect("upgrade");
        assert!(!marker.exists(), "upgrade must replace the extension dir");
        let version = installed_manifest_version(&installed_dir.join("lex")).expect("version");
        assert_eq!(version, bundled_manifest_version("lex").expect("bundled"));
    }

    #[test]
    fn leaves_dev_extension_symlinks_alone() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let installed_dir = temp_dir.path().join("installed");
        std::fs::create_dir_all(&installed_dir).expect("create installed dir");

        let dev_extension_dir = temp_dir.path().join("dev-lex");
        std::fs::create_dir_all(&dev_extension_dir).expect("create dev dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&dev_extension_dir, installed_dir.join("lex"))
            .expect("symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&dev_extension_dir, installed_dir.join("lex"))
            .expect("symlink");

        install_extension(&installed_dir, "lex").expect("install");
        assert!(
            installed_dir.join("lex").symlink_metadata().unwrap().is_symlink(),
            "dev extension symlink must be preserved"
        );
        assert!(!installed_dir.join("lex/extension.wasm").exists());
    }
}
