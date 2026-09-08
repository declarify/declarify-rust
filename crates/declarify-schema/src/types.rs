//! Collects local struct/enum definitions and resolves the supported Step 0
//! field type subset (`String` and `bool` only).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use syn::punctuated::Punctuated;
use syn::{Item, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Bool,
}

pub struct LocalField {
    pub name: String,
    pub ty: syn::Type,
    pub span: proc_macro2::Span,
    pub has_serde_attr: bool,
}

pub enum LocalTypeKind {
    /// A struct with named fields.
    Struct(Vec<LocalField>),
    /// A struct without named fields (unit or tuple) — not a valid Spec/Status root.
    UnsupportedStructShape,
    Enum,
}

pub struct LocalType {
    pub file: PathBuf,
    pub name_span: proc_macro2::Span,
    pub derives: Vec<String>,
    pub kind: LocalTypeKind,
}

/// Collects every local struct/enum across the given parsed files, keyed by
/// simple name. A `Vec` of length > 1 for a key means the name is ambiguous
/// within the scanned package.
pub fn collect_local_types(files: &[(PathBuf, syn::File)]) -> HashMap<String, Vec<LocalType>> {
    let mut out: HashMap<String, Vec<LocalType>> = HashMap::new();
    for (path, file) in files {
        collect_items(&file.items, path, &mut out);
    }
    out
}

fn collect_items(items: &[Item], file: &Path, out: &mut HashMap<String, Vec<LocalType>>) {
    for item in items {
        match item {
            Item::Struct(item_struct) => {
                let derives = collect_derives(&item_struct.attrs);
                let kind = match &item_struct.fields {
                    syn::Fields::Named(named) => LocalTypeKind::Struct(
                        named
                            .named
                            .iter()
                            .map(|f| LocalField {
                                name: f.ident.as_ref().unwrap().to_string(),
                                ty: f.ty.clone(),
                                span: f.ident.as_ref().unwrap().span(),
                                has_serde_attr: f.attrs.iter().any(|a| a.path().is_ident("serde")),
                            })
                            .collect(),
                    ),
                    syn::Fields::Unnamed(_) | syn::Fields::Unit => {
                        LocalTypeKind::UnsupportedStructShape
                    }
                };
                out.entry(item_struct.ident.to_string())
                    .or_default()
                    .push(LocalType {
                        file: file.to_path_buf(),
                        name_span: item_struct.ident.span(),
                        derives,
                        kind,
                    });
            }
            Item::Enum(item_enum) => {
                let derives = collect_derives(&item_enum.attrs);
                out.entry(item_enum.ident.to_string())
                    .or_default()
                    .push(LocalType {
                        file: file.to_path_buf(),
                        name_span: item_enum.ident.span(),
                        derives,
                        kind: LocalTypeKind::Enum,
                    });
            }
            Item::Mod(item_mod) => {
                if let Some((_, inline_items)) = &item_mod.content {
                    collect_items(inline_items, file, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_derives(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut derives = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        if let Ok(paths) =
            attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
        {
            for path in paths {
                if let Some(seg) = path.segments.last() {
                    derives.push(seg.ident.to_string());
                }
            }
        }
    }
    derives
}

/// Resolves the supported Step 0 field type subset. Anything else is an
/// explicit declaration error naming the offending type.
pub fn resolve_field_type(ty: &syn::Type) -> Result<FieldType, String> {
    if let syn::Type::Path(type_path) = ty {
        if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
            let seg = type_path.path.segments.last().expect("checked len == 1");
            if seg.arguments.is_empty() {
                match seg.ident.to_string().as_str() {
                    "String" => return Ok(FieldType::String),
                    "bool" => return Ok(FieldType::Bool),
                    _ => {}
                }
            }
        }
    }
    Err(format!(
        "unsupported field type `{}`; Step 0 supports only `String` and `bool`",
        quote::quote!(#ty)
    ))
}
