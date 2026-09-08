//! Constrained Rust source scanner and validated schema model for Declarify
//! Step 0.
//!
//! This is not a general Rust compiler front-end. It scans a Cargo package's
//! handwritten sources for `declarify::resource!` declarations, resolves the
//! referenced local `Spec`/`Status` structs, and validates them against the
//! deliberately narrow Step 0 field subset (`String` and `bool`). See
//! `docs/02-architecture.md` and `docs/01-resource-model.md` for the product
//! rules this module implements.

mod declaration;
mod discovery;
mod types;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub use discovery::{collect_rust_sources, locate_package, DiscoveryError, PackageLocation};
pub use types::FieldType;

/// A validated resource, ready for code generation.
#[derive(Debug, Clone)]
pub struct ResourceSchema {
    pub name: String,
    pub api_version: String,
    pub plural: String,
    pub operations: Vec<declarify_core::Operation>,
    pub spec: TypeSchema,
    pub status: TypeSchema,
}

/// A validated `Spec` or `Status` shape: a local struct with named fields
/// drawn from the supported Step 0 type subset.
#[derive(Debug, Clone)]
pub struct TypeSchema {
    pub name: String,
    /// The `::`-separated module path (relative to the crate root, no
    /// leading/trailing `::`) where this type is defined, e.g. `domain::note`,
    /// or the empty string if it is defined directly at the crate root. This
    /// assumes the standard Cargo directory-per-module convention; a consumer
    /// using non-standard `#[path]` attributes is outside Step 0's scanner
    /// support. See `docs/09-decisions.md`.
    pub module_path: String,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub ty: FieldType,
}

/// One actionable validation failure, naming the resource/type/field and the
/// reason, with a source location when one is available.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.file, self.line) {
            (Some(file), Some(line)) => write!(f, "{}:{line}: {}", file.display(), self.message),
            (Some(file), None) => write!(f, "{}: {}", file.display(), self.message),
            (None, _) => write!(f, "{}", self.message),
        }
    }
}

/// The result of scanning one Cargo package's source tree: every resource
/// that validated cleanly, plus every diagnostic collected along the way. A
/// resource with any invalid field/type is omitted from `resources`, not
/// partially included.
#[derive(Debug, Default)]
pub struct ScanOutcome {
    pub resources: Vec<ResourceSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScanOutcome {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Discovers `package` via `cargo metadata` and scans its source tree.
pub fn validate_package(package: &str) -> Result<ScanOutcome, DiscoveryError> {
    let location = discovery::locate_package(package)?;
    Ok(scan_source_root(&location.src_dir))
}

/// Scans every `.rs` file under `root` (excluding `generated/` directories)
/// for resource declarations and validates them.
pub fn scan_source_root(root: &Path) -> ScanOutcome {
    let mut diagnostics = Vec::new();

    let files = match discovery::collect_rust_sources(root) {
        Ok(files) => files,
        Err(err) => {
            diagnostics.push(Diagnostic {
                file: Some(root.to_path_buf()),
                line: None,
                message: format!("failed to read source directory: {err}"),
            });
            return ScanOutcome {
                resources: Vec::new(),
                diagnostics,
            };
        }
    };

    let mut parsed: Vec<(PathBuf, syn::File)> = Vec::new();
    for path in files {
        match std::fs::read_to_string(&path) {
            Ok(source) => match syn::parse_file(&source) {
                Ok(file) => parsed.push((path, file)),
                Err(err) => diagnostics.push(Diagnostic {
                    file: Some(path),
                    line: Some(err.span().start().line),
                    message: format!("failed to parse Rust source: {err}"),
                }),
            },
            Err(err) => diagnostics.push(Diagnostic {
                file: Some(path),
                line: None,
                message: format!("failed to read file: {err}"),
            }),
        }
    }

    let local_types = types::collect_local_types(&parsed);

    let mut raw_decls: Vec<(PathBuf, declaration::RawResourceDecl)> = Vec::new();
    for (path, file) in &parsed {
        find_resource_macros(&file.items, path, &mut raw_decls, &mut diagnostics);
    }

    let mut seen_names: HashMap<String, PathBuf> = HashMap::new();
    let mut resources = Vec::new();

    for (path, raw) in raw_decls {
        if let Some(prev_file) = seen_names.get(&raw.name) {
            diagnostics.push(Diagnostic {
                file: Some(path.clone()),
                line: Some(raw.name_span.start().line),
                message: format!(
                    "duplicate resource name `{}` (already declared in {})",
                    raw.name,
                    prev_file.display()
                ),
            });
            continue;
        }
        seen_names.insert(raw.name.clone(), path.clone());

        let mut ok = true;

        let mut operations = Vec::new();
        for op in &raw.operations {
            match declarify_core::Operation::parse(&op.keyword) {
                Some(parsed_op) => operations.push(parsed_op),
                None => {
                    ok = false;
                    let allowed = declarify_core::Operation::ALL
                        .iter()
                        .map(|o| o.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    diagnostics.push(Diagnostic {
                        file: Some(path.clone()),
                        line: Some(op.span.start().line),
                        message: format!(
                            "resource `{}`: unknown operation `{}`; expected one of: {allowed}",
                            raw.name, op.keyword
                        ),
                    });
                }
            }
        }

        let spec = resolve_type_schema(
            &raw.name,
            "spec",
            &raw.spec,
            raw.spec_span,
            &path,
            root,
            &local_types,
            &mut diagnostics,
        );
        let status = resolve_type_schema(
            &raw.name,
            "status",
            &raw.status,
            raw.status_span,
            &path,
            root,
            &local_types,
            &mut diagnostics,
        );

        let (Some(spec), Some(status)) = (spec, status) else {
            continue;
        };
        if !ok {
            continue;
        }

        resources.push(ResourceSchema {
            name: raw.name,
            api_version: raw.api_version,
            plural: raw.plural,
            operations,
            spec,
            status,
        });
    }

    resources.sort_by(|a, b| a.name.cmp(&b.name));

    ScanOutcome {
        resources,
        diagnostics,
    }
}

fn find_resource_macros(
    items: &[syn::Item],
    file: &Path,
    out: &mut Vec<(PathBuf, declaration::RawResourceDecl)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            syn::Item::Macro(item_macro) => {
                if !declaration::is_resource_macro_path(&item_macro.mac.path) {
                    continue;
                }
                match declaration::parse_resource_macro(item_macro.mac.tokens.clone()) {
                    Ok(decl) => out.push((file.to_path_buf(), decl)),
                    Err(err) => diagnostics.push(Diagnostic {
                        file: Some(file.to_path_buf()),
                        line: Some(err.span().start().line),
                        message: err.to_string(),
                    }),
                }
            }
            syn::Item::Mod(item_mod) => {
                if let Some((_, inline_items)) = &item_mod.content {
                    find_resource_macros(inline_items, file, out, diagnostics);
                }
            }
            _ => {}
        }
    }
}

/// Converts a scanned file's path (relative to the scanned `root`) into a
/// `::`-separated Rust module path, following the standard Cargo
/// directory-per-module convention: `domain/note.rs` -> `domain::note`,
/// `domain/mod.rs` -> `domain`, `lib.rs`/`main.rs` at the root -> `` (empty).
fn module_path_for(root: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(root).unwrap_or(file).with_extension("");
    let mut segments: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if matches!(segments.last().map(String::as_str), Some("mod")) {
        segments.pop();
    }
    if segments.len() == 1 && matches!(segments[0].as_str(), "lib" | "main") {
        segments.pop();
    }
    segments.join("::")
}

#[allow(clippy::too_many_arguments)]
fn resolve_type_schema(
    resource_name: &str,
    role: &str,
    type_name: &str,
    span: proc_macro2::Span,
    decl_file: &Path,
    root: &Path,
    local_types: &HashMap<String, Vec<types::LocalType>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TypeSchema> {
    let candidates = match local_types.get(type_name) {
        Some(c) if !c.is_empty() => c,
        _ => {
            diagnostics.push(Diagnostic {
                file: Some(decl_file.to_path_buf()),
                line: Some(span.start().line),
                message: format!(
                    "resource `{resource_name}`: referenced {role} type `{type_name}` was not \
                     found in the scanned source"
                ),
            });
            return None;
        }
    };

    if candidates.len() > 1 {
        let files = candidates
            .iter()
            .map(|c| c.file.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        diagnostics.push(Diagnostic {
            file: Some(decl_file.to_path_buf()),
            line: Some(span.start().line),
            message: format!(
                "resource `{resource_name}`: referenced {role} type `{type_name}` is ambiguous \
                 (found in: {files})"
            ),
        });
        return None;
    }

    let local_type = &candidates[0];
    let mut ok = true;

    let has_serialize = local_type.derives.iter().any(|d| d == "Serialize");
    let has_deserialize = local_type.derives.iter().any(|d| d == "Deserialize");
    if !has_serialize || !has_deserialize {
        ok = false;
        diagnostics.push(Diagnostic {
            file: Some(local_type.file.clone()),
            line: Some(local_type.name_span.start().line),
            message: format!(
                "type `{type_name}` (used as {role} of resource `{resource_name}`) must derive \
                 both `serde::Serialize` and `serde::Deserialize`"
            ),
        });
    }

    if role == "status" && !local_type.derives.iter().any(|d| d == "Default") {
        ok = false;
        diagnostics.push(Diagnostic {
            file: Some(local_type.file.clone()),
            line: Some(local_type.name_span.start().line),
            message: format!(
                "type `{type_name}` (used as status of resource `{resource_name}`) must derive \
                 `Default`; Step 0 produces initial status from `Default` (docs/01-resource-model.md)"
            ),
        });
    }

    let fields = match &local_type.kind {
        types::LocalTypeKind::Enum => {
            ok = false;
            diagnostics.push(Diagnostic {
                file: Some(local_type.file.clone()),
                line: Some(local_type.name_span.start().line),
                message: format!(
                    "type `{type_name}` (used as {role} of resource `{resource_name}`) must be a \
                     struct with named fields; enums are not yet supported as a Spec/Status root type"
                ),
            });
            Vec::new()
        }
        types::LocalTypeKind::UnsupportedStructShape => {
            ok = false;
            diagnostics.push(Diagnostic {
                file: Some(local_type.file.clone()),
                line: Some(local_type.name_span.start().line),
                message: format!(
                    "type `{type_name}` (used as {role} of resource `{resource_name}`) must be a \
                     struct with named fields"
                ),
            });
            Vec::new()
        }
        types::LocalTypeKind::Struct(local_fields) => {
            let mut fields = Vec::new();
            for field in local_fields {
                if field.has_serde_attr {
                    ok = false;
                    diagnostics.push(Diagnostic {
                        file: Some(local_type.file.clone()),
                        line: Some(field.span.start().line),
                        message: format!(
                            "field `{}` of type `{type_name}` ({role} of resource \
                             `{resource_name}`): `#[serde(..)]` field attributes are not yet \
                             supported by the Step 0 scanner",
                            field.name
                        ),
                    });
                    continue;
                }
                match types::resolve_field_type(&field.ty) {
                    Ok(ty) => fields.push(FieldSchema {
                        name: field.name.clone(),
                        ty,
                    }),
                    Err(reason) => {
                        ok = false;
                        diagnostics.push(Diagnostic {
                            file: Some(local_type.file.clone()),
                            line: Some(field.span.start().line),
                            message: format!(
                                "field `{}` of type `{type_name}` ({role} of resource \
                                 `{resource_name}`): {reason}",
                                field.name
                            ),
                        });
                    }
                }
            }
            fields
        }
    };

    if !ok {
        return None;
    }

    Some(TypeSchema {
        name: type_name.to_string(),
        module_path: module_path_for(root, &local_type.file),
        fields,
    })
}

#[cfg(test)]
mod module_path_tests {
    use super::module_path_for;
    use std::path::Path;

    #[test]
    fn nested_file_maps_to_dotted_module_path() {
        let root = Path::new("/pkg/src");
        assert_eq!(
            module_path_for(root, Path::new("/pkg/src/domain/note.rs")),
            "domain::note"
        );
    }

    #[test]
    fn mod_rs_maps_to_its_directory() {
        let root = Path::new("/pkg/src");
        assert_eq!(
            module_path_for(root, Path::new("/pkg/src/domain/mod.rs")),
            "domain"
        );
    }

    #[test]
    fn crate_root_file_maps_to_empty_path() {
        let root = Path::new("/pkg/src");
        assert_eq!(module_path_for(root, Path::new("/pkg/src/lib.rs")), "");
    }
}
