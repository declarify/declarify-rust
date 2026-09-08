//! Implements `declarify generate` and `declarify check`. See
//! `docs/03-declaration-and-generation.md` ("Determinism and safety") for the
//! exact sequence this must follow: validate, render to a temp directory,
//! format, compile-check, then atomically replace the real generated tree —
//! and never touch it at all if any earlier step fails.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use declarify_codegen::GeneratedTree;

fn validated_resources(
    package: &str,
) -> anyhow::Result<(
    declarify_schema::PackageLocation,
    Vec<declarify_schema::ResourceSchema>,
)> {
    let location = declarify_schema::locate_package(package)?;
    let outcome = declarify_schema::scan_source_root(&location.src_dir);
    if !outcome.diagnostics.is_empty() {
        for diagnostic in &outcome.diagnostics {
            eprintln!("error: {diagnostic}");
        }
        anyhow::bail!(
            "{} validation error(s) in package `{package}`; generated tree left untouched",
            outcome.diagnostics.len()
        );
    }
    Ok((location, outcome.resources))
}

fn write_tree(root: &Path, tree: &GeneratedTree) -> anyhow::Result<()> {
    for (relative, contents) in tree {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
    }
    Ok(())
}

fn format_files(root: &Path, relative_paths: impl Iterator<Item = PathBuf>) -> anyhow::Result<()> {
    let mut cmd = Command::new("rustfmt");
    cmd.arg("--edition").arg("2021");
    let mut any = false;
    for relative in relative_paths {
        cmd.arg(root.join(relative));
        any = true;
    }
    if !any {
        return Ok(());
    }
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("rustfmt failed while formatting generated code");
    }
    Ok(())
}

pub fn run_generate(package: &str) -> anyhow::Result<()> {
    let (location, resources) = validated_resources(package)?;
    let tree = declarify_codegen::generate(&resources);

    let temp_dir = location.src_dir.join(".generated.tmp");
    let real_dir = location.src_dir.join("generated");
    let backup_dir = location.src_dir.join(".generated.bak");

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    write_tree(&temp_dir, &tree)?;
    format_files(&temp_dir, tree.keys().cloned())?;

    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir)?;
    }
    let had_previous = real_dir.exists();
    if had_previous {
        fs::rename(&real_dir, &backup_dir)?;
    }
    fs::rename(&temp_dir, &real_dir)?;

    let compile = Command::new("cargo")
        .args(["build", "-p", package])
        .current_dir(&location.workspace_root)
        .output()?;

    if compile.status.success() {
        if had_previous {
            fs::remove_dir_all(&backup_dir)?;
        }
        println!(
            "declarify generate: wrote {} file(s) to {}",
            tree.len(),
            real_dir.display()
        );
        Ok(())
    } else {
        // The new tree does not compile: restore exactly what was there
        // before, so a failed run leaves the previous generated tree
        // untouched.
        fs::remove_dir_all(&real_dir)?;
        if had_previous {
            fs::rename(&backup_dir, &real_dir)?;
        }
        eprint!("{}", String::from_utf8_lossy(&compile.stderr));
        anyhow::bail!(
            "generated code for package `{package}` failed to compile; previous generated tree restored"
        );
    }
}

pub fn run_check(package: &str) -> anyhow::Result<()> {
    let (location, resources) = validated_resources(package)?;
    let tree = declarify_codegen::generate(&resources);

    let temp_dir = location.src_dir.join(".generated.check-tmp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    write_tree(&temp_dir, &tree)?;
    format_files(&temp_dir, tree.keys().cloned())?;

    let real_dir = location.src_dir.join("generated");
    let drift = diff_trees(&real_dir, &temp_dir)?;
    fs::remove_dir_all(&temp_dir)?;

    if drift.is_empty() {
        println!("declarify check: generated tree matches a fresh generation");
        Ok(())
    } else {
        for line in &drift {
            eprintln!("{line}");
        }
        anyhow::bail!(
            "declarify check: {} file(s) differ from a fresh generation for package `{package}`",
            drift.len()
        );
    }
}

fn diff_trees(committed: &Path, fresh: &Path) -> anyhow::Result<Vec<String>> {
    let mut diffs = Vec::new();

    let committed_files = if committed.exists() {
        declarify_schema::collect_rust_sources(committed)?
    } else {
        Vec::new()
    };
    let fresh_files = declarify_schema::collect_rust_sources(fresh)?;

    let committed_rel: BTreeSet<PathBuf> = committed_files
        .iter()
        .map(|p| p.strip_prefix(committed).unwrap_or(p).to_path_buf())
        .collect();
    let fresh_rel: BTreeSet<PathBuf> = fresh_files
        .iter()
        .map(|p| p.strip_prefix(fresh).unwrap_or(p).to_path_buf())
        .collect();

    for missing in fresh_rel.difference(&committed_rel) {
        diffs.push(format!(
            "missing from committed generated tree: {}",
            missing.display()
        ));
    }
    for extra in committed_rel.difference(&fresh_rel) {
        diffs.push(format!(
            "stale file in committed generated tree (no longer generated): {}",
            extra.display()
        ));
    }
    for common in fresh_rel.intersection(&committed_rel) {
        let committed_contents = fs::read_to_string(committed.join(common))?;
        let fresh_contents = fs::read_to_string(fresh.join(common))?;
        if committed_contents != fresh_contents {
            diffs.push(format!(
                "content differs from a fresh generation: {}",
                common.display()
            ));
        }
    }
    Ok(diffs)
}
