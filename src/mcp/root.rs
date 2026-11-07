//! Approved directory for MCP scans.
//!
//! `tracelamp scan` is unchanged. The MCP tools refuse a path until a root is
//! configured, and they refuse a path that canonicalizes outside that root.

use crate::config::TraceLampConfig;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

const SCAN_ROOT_ENV: &str = "TRACELAMP_SCAN_ROOT";

pub fn contain(config: &TraceLampConfig, requested: &str) -> Result<PathBuf> {
    let root = configured_root(config)?;
    let root = std::fs::canonicalize(&root)
        .with_context(|| format!("scan root is missing or invalid: {}", root.display()))?;
    if !root.is_dir() {
        bail!("scan root is not a directory: {}", root.display());
    }
    let requested_path = Path::new(requested);
    let resolved = std::fs::canonicalize(requested_path).with_context(|| {
        format!(
            "scan path is missing or invalid: {}",
            requested_path.display()
        )
    })?;
    if resolved != root && !resolved.starts_with(&root) {
        bail!(
            "scan path is outside the approved root {}: {}",
            root.display(),
            resolved.display()
        );
    }
    Ok(resolved)
}

fn configured_root(config: &TraceLampConfig) -> Result<PathBuf> {
    if let Some(root) = config
        .mcp
        .scan_root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
    {
        return Ok(PathBuf::from(root));
    }
    if let Some(root) = std::env::var(SCAN_ROOT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|root| !root.is_empty())
    {
        return Ok(PathBuf::from(root));
    }
    bail!("scan root is not configured. Set mcp.scan_root or {SCAN_ROOT_ENV} to an absolute directory.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TraceLampConfig;
    use std::os::unix::fs::symlink;

    fn config_with(root: &Path) -> TraceLampConfig {
        let mut config = TraceLampConfig::default();
        config.mcp.scan_root = Some(root.display().to_string());
        config
    }

    #[test]
    fn missing_root_is_rejected() {
        let previous = std::env::var(SCAN_ROOT_ENV).ok();
        std::env::remove_var(SCAN_ROOT_ENV);
        let dir = tempfile::tempdir().unwrap();
        let err = contain(&TraceLampConfig::default(), dir.path().to_str().unwrap()).unwrap_err();
        if let Some(previous) = previous {
            std::env::set_var(SCAN_ROOT_ENV, previous);
        }
        assert!(err.to_string().contains("not configured"), "{err}");
    }

    #[test]
    fn file_root_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, "x").unwrap();
        let err = contain(&config_with(&file), file.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("not a directory"), "{err}");
    }

    #[test]
    fn symlink_and_parent_escape_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "nope").unwrap();
        symlink(outside.path(), root.path().join("escape")).unwrap();
        let config = config_with(root.path());
        let err = contain(&config, root.path().join("escape").to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("outside"), "{err}");

        let parent = root.path().join("sub").join("..").join("..");
        std::fs::create_dir(root.path().join("sub")).unwrap();
        let resolved = contain(&config, root.path().join("sub").to_str().unwrap());
        assert!(resolved.is_ok(), "{resolved:?}");
        let escaped = contain(&config, parent.to_str().unwrap()).unwrap_err();
        assert!(escaped.to_string().contains("outside"), "{escaped}");
    }
}
