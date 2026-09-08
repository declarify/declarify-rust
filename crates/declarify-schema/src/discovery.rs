//! Locates a Cargo package's handwritten source root.

use std::fmt;
use std::path::{Path, PathBuf};

/// Failure to locate a package's source root.
#[derive(Debug)]
pub enum DiscoveryError {
    Metadata(cargo_metadata::Error),
    PackageNotFound { package: String },
    SrcDirMissing { package: String, expected: PathBuf },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::Metadata(err) => write!(f, "failed to run `cargo metadata`: {err}"),
            DiscoveryError::PackageNotFound { package } => {
                write!(f, "no workspace package named `{package}` was found")
            }
            DiscoveryError::SrcDirMissing { package, expected } => write!(
                f,
                "package `{package}` has no `src` directory (expected {})",
                expected.display()
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Where a Cargo package lives: its handwritten `src/` directory, and the
/// workspace root (needed by `declarify generate`/`check` to invoke `cargo`
/// for the post-generation compile check).
#[derive(Debug, Clone)]
pub struct PackageLocation {
    pub workspace_root: PathBuf,
    pub src_dir: PathBuf,
}

/// Resolves `package` (a Cargo package name) using `cargo metadata`, so
/// `declarify` can be invoked from anywhere inside the workspace without
/// hardcoding paths.
pub fn locate_package(package: &str) -> Result<PackageLocation, DiscoveryError> {
    locate_package_from(package, None)
}

/// Same as [`locate_package`], but runs `cargo metadata` rooted at an
/// explicit manifest directory. Exposed for tests that must not depend on the
/// ambient working directory.
pub fn locate_package_from(
    package: &str,
    manifest_dir: Option<&Path>,
) -> Result<PackageLocation, DiscoveryError> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(dir) = manifest_dir {
        cmd.manifest_path(dir.join("Cargo.toml"));
    }
    let metadata = cmd.exec().map_err(DiscoveryError::Metadata)?;

    let pkg = metadata
        .packages
        .iter()
        .find(|p| p.name == package)
        .ok_or_else(|| DiscoveryError::PackageNotFound {
            package: package.to_string(),
        })?;

    let manifest_dir = pkg
        .manifest_path
        .parent()
        .expect("manifest path always has a parent directory");
    let src_dir = manifest_dir.join("src");
    if !src_dir.is_dir() {
        return Err(DiscoveryError::SrcDirMissing {
            package: package.to_string(),
            expected: src_dir.clone().into_std_path_buf(),
        });
    }
    Ok(PackageLocation {
        workspace_root: metadata.workspace_root.into_std_path_buf(),
        src_dir: src_dir.into_std_path_buf(),
    })
}

/// Recursively collects every `.rs` file under `root`, skipping the
/// generator-owned `generated` directory and any hidden (dot-prefixed)
/// directory, e.g. the temp/backup directories `declarify generate`/`check`
/// use while rendering.
pub fn collect_rust_sources(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "generated" || name.starts_with('.') {
                continue;
            }
            walk(&path, files)?;
        } else if file_type.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}
