//! Directory walk that honors `.gitignore` and `.reconignore`.
//!
//! Default directories are always skipped. Hidden files such as `.mcp.json`
//! and `.cursor/` are still visited. The walk only reads the filesystem.

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEFAULT_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "vendor",
    ".go",
    "dist",
    "build",
    "__pycache__",
    ".venv",
];

pub fn list_files(root: &Path, extra_patterns: &[&str]) -> Result<Vec<PathBuf>> {
    let extra = Arc::new(build_extra(root, extra_patterns)?);
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .parents(true)
        .ignore(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .require_git(false)
        .add_custom_ignore_filename(".reconignore")
        .filter_entry({
            let extra = Arc::clone(&extra);
            move |entry| {
                if entry.depth() == 0 {
                    return true;
                }
                let name = entry.file_name().to_string_lossy();
                if DEFAULT_DIRS.contains(&name.as_ref()) {
                    return false;
                }
                let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
                !extra.matched(entry.path(), is_dir).is_ignore()
            }
        })
        .build();

    let mut files = Vec::new();
    for entry in walker {
        match entry {
            Ok(entry) => {
                if entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
                {
                    files.push(entry.into_path());
                }
            }
            Err(err) => {
                tracing::debug!("skip walk entry: {err}");
            }
        }
    }
    Ok(files)
}

fn build_extra(root: &Path, patterns: &[&str]) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            continue;
        }
        builder
            .add_line(None, pattern)
            .with_context(|| format!("invalid ignore pattern: {pattern}"))?;
    }
    builder.build().context("build ignore patterns")
}

pub fn is_binary_extension(path: &Path) -> bool {
    const EXTS: &[&str] = &[
        "exe", "dll", "so", "dylib", "bin", "o", "a", "obj", "lib", "class", "jar", "war", "pyc",
        "pyo", "wasm", "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "pdf", "zip", "tar",
        "gz", "bz2", "7z", "xz", "rar", "woff", "woff2", "ttf", "eot", "otf", "mp3", "mp4", "mov",
        "avi", "mkv", "webm", "sqlite", "db",
    ];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| EXTS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn looks_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dirs_and_ignore_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("kept.py"), "print(1)\n").unwrap();
        std::fs::write(root.join(".gitignore"), "generated.py\n").unwrap();
        std::fs::write(root.join(".reconignore"), "local_skip.py\n").unwrap();
        std::fs::write(root.join("generated.py"), "print(2)\n").unwrap();
        std::fs::write(root.join("local_skip.py"), "print(3)\n").unwrap();
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::write(root.join("node_modules/pkg/index.js"), "1\n").unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/config"), "x\n").unwrap();
        std::fs::create_dir_all(root.join(".cursor")).unwrap();
        std::fs::write(root.join(".cursor/mcp.json"), "{}\n").unwrap();
        std::fs::write(root.join(".github_not_git.txt"), "ok\n").unwrap();

        let files = list_files(root, &[]).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert!(names.iter().any(|name| name == "kept.py"), "{names:?}");
        assert!(
            names.iter().any(|name| name == ".cursor/mcp.json"),
            "{names:?}"
        );
        assert!(
            names.iter().any(|name| name == ".github_not_git.txt"),
            "{names:?}"
        );
        assert!(
            !names.iter().any(|name| name.contains("node_modules")),
            "{names:?}"
        );
        assert!(
            !names.iter().any(|name| name.starts_with(".git/")),
            "{names:?}"
        );
        assert!(
            !names.iter().any(|name| name == "generated.py"),
            "{names:?}"
        );
        assert!(
            !names.iter().any(|name| name == "local_skip.py"),
            "{names:?}"
        );
    }
}
