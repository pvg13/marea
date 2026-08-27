//! Putting a [`Plan`] on disk.
//!
//! Deliberately the only module that writes files, and deliberately dumb: the
//! interesting decisions all happened in `render`, where they are testable
//! without a filesystem.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::render::Plan;

/// Write every planned file under `root`, creating parent directories.
///
/// Refuses to write into a directory that already has contents unless
/// `force` is set — scaffolding over an existing project is almost always a
/// mistake, and an overwritten `src/` is not something git can help with if
/// the project was never committed.
pub fn write_plan(plan: &Plan, root: &Path, force: bool) -> Result<Vec<PathBuf>> {
    if root.exists() {
        let non_empty = std::fs::read_dir(root)
            .with_context(|| format!("reading {}", root.display()))?
            .next()
            .is_some();
        if non_empty && !force {
            bail!(
                "{} already exists and is not empty (pass --force to write into it anyway)",
                root.display()
            );
        }
    }

    let mut written = Vec::with_capacity(plan.files.len());
    for file in &plan.files {
        let dest = root.join(&file.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&dest, &file.contents)
            .with_context(|| format!("writing {}", dest.display()))?;
        written.push(dest);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::PlannedFile;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("marea-cli-write-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn plan() -> Plan {
        Plan {
            files: vec![
                PlannedFile {
                    path: "Cargo.toml".into(),
                    contents: "[workspace]\n".into(),
                },
                PlannedFile {
                    path: "crates/ui/src/lib.rs".into(),
                    contents: "// hi\n".into(),
                },
            ],
        }
    }

    #[test]
    fn creates_nested_directories() {
        let root = scratch("nested");
        write_plan(&plan(), &root, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("crates/ui/src/lib.rs")).unwrap(),
            "// hi\n"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn refuses_to_scaffold_over_an_existing_project() {
        let root = scratch("existing");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("README.md"), "mine").unwrap();

        let err = write_plan(&plan(), &root, false).unwrap_err();
        assert!(
            err.to_string().contains("already exists and is not empty"),
            "unexpected error: {err}"
        );
        // and left the existing file alone
        assert_eq!(
            std::fs::read_to_string(root.join("README.md")).unwrap(),
            "mine"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn force_writes_into_a_non_empty_directory() {
        let root = scratch("forced");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("README.md"), "mine").unwrap();

        write_plan(&plan(), &root, true).unwrap();
        assert!(root.join("Cargo.toml").is_file());
        std::fs::remove_dir_all(&root).ok();
    }
}
