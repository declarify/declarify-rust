//! Implements the `declarify::resource!` declaration surface.
//!
//! This macro exists to make a Declarify resource declaration valid,
//! immediately-diagnosed Rust syntax. It deliberately expands to nothing: the
//! authoritative schema used for validation/generation is produced by
//! `declarify-schema`, which re-scans the declaration from source text so
//! that `declarify validate`/`generate` can run without compiling the
//! consumer crate. Keeping the macro a pure compile-time syntax/keyword check
//! (rather than an expansion consumers depend on) means the two readings of
//! the declaration can never silently drift: both are driven by the same
//! fixed grammar and the same `declarify_core::Operation` vocabulary.

use proc_macro::TokenStream;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, LitStr, Token, Visibility,
};

#[cfg_attr(test, derive(Debug))]
struct ResourceDecl {
    #[allow(dead_code)]
    vis: Visibility,
    #[allow(dead_code)]
    name: Ident,
}

/// One `key: value` field inside a resource declaration body.
enum Field {
    ApiVersion(LitStr),
    Plural(LitStr),
    Spec(Ident),
    Status(Ident),
    Operations(Vec<Ident>),
}

impl Parse for ResourceDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis: Visibility = input.parse()?;
        let name: Ident = input.parse()?;

        let body;
        braced!(body in input);
        let fields: Punctuated<Field, Token![,]> =
            body.parse_terminated(Field::parse, Token![,])?;

        let mut api_version = None;
        let mut plural = None;
        let mut spec = None;
        let mut status = None;
        let mut operations = None;

        for field in fields {
            match field {
                Field::ApiVersion(v) => api_version = Some(v),
                Field::Plural(v) => plural = Some(v),
                Field::Spec(v) => spec = Some(v),
                Field::Status(v) => status = Some(v),
                Field::Operations(v) => operations = Some(v),
            }
        }

        let mut missing = Vec::new();
        if api_version.is_none() {
            missing.push("api_version");
        }
        if plural.is_none() {
            missing.push("plural");
        }
        if spec.is_none() {
            missing.push("spec");
        }
        if status.is_none() {
            missing.push("status");
        }
        let operations = match operations {
            Some(ops) => ops,
            None => {
                missing.push("operations");
                Vec::new()
            }
        };
        if !missing.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "resource `{}` is missing required field(s): {}",
                    name,
                    missing.join(", ")
                ),
            ));
        }
        if operations.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                format!("resource `{name}` must enable at least one operation"),
            ));
        }
        for op in &operations {
            if declarify_core::Operation::parse(&op.to_string()).is_none() {
                let allowed = declarify_core::Operation::ALL
                    .iter()
                    .map(|o| o.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(syn::Error::new(
                    op.span(),
                    format!("unknown operation `{op}`; expected one of: {allowed}"),
                ));
            }
        }

        Ok(ResourceDecl { vis, name })
    }
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        match key.to_string().as_str() {
            "api_version" => Ok(Field::ApiVersion(input.parse()?)),
            "plural" => Ok(Field::Plural(input.parse()?)),
            "spec" => Ok(Field::Spec(input.parse()?)),
            "status" => Ok(Field::Status(input.parse()?)),
            "operations" => {
                let content;
                bracketed!(content in input);
                let ops: Punctuated<Ident, Token![,]> =
                    content.parse_terminated(Ident::parse, Token![,])?;
                Ok(Field::Operations(ops.into_iter().collect()))
            }
            other => Err(syn::Error::new(
                key.span(),
                format!(
                    "unknown resource declaration field `{other}`; expected one of: \
                     api_version, plural, spec, status, operations"
                ),
            )),
        }
    }
}

/// Declares a Declarify resource.
///
/// See `docs/03-declaration-and-generation.md` for the full grammar. This
/// macro only validates the declaration; it produces no output items.
#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    let _decl = parse_macro_input!(input as ResourceDecl);
    TokenStream::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn parse(tokens: proc_macro2::TokenStream) -> syn::Result<ResourceDecl> {
        syn::parse2(tokens)
    }

    #[test]
    fn accepts_the_canonical_note_declaration() {
        let decl = parse(quote! {
            pub Note {
                api_version: "notes.declarify.dev/v1",
                plural: "notes",
                spec: NoteSpec,
                status: NoteStatus,
                operations: [create, get, list, update_spec, update_status, delete],
            }
        })
        .expect("valid declaration must parse");
        assert_eq!(decl.name.to_string(), "Note");
    }

    #[test]
    fn rejects_missing_required_field() {
        let err = parse(quote! {
            pub Note {
                api_version: "notes.declarify.dev/v1",
                plural: "notes",
                spec: NoteSpec,
                operations: [create],
            }
        })
        .expect_err("status is missing");
        assert!(err.to_string().contains("status"));
    }

    #[test]
    fn rejects_unknown_operation_keyword() {
        let err = parse(quote! {
            pub Note {
                api_version: "notes.declarify.dev/v1",
                plural: "notes",
                spec: NoteSpec,
                status: NoteStatus,
                operations: [create, archive],
            }
        })
        .expect_err("archive is not a Step 0 operation");
        assert!(err.to_string().contains("archive"));
    }

    #[test]
    fn rejects_empty_operations_list() {
        let err = parse(quote! {
            pub Note {
                api_version: "notes.declarify.dev/v1",
                plural: "notes",
                spec: NoteSpec,
                status: NoteStatus,
                operations: [],
            }
        })
        .expect_err("at least one operation is required");
        assert!(err.to_string().contains("at least one operation"));
    }

    #[test]
    fn rejects_unknown_declaration_field() {
        let err = parse(quote! {
            pub Note {
                api_version: "notes.declarify.dev/v1",
                plural: "notes",
                spec: NoteSpec,
                status: NoteStatus,
                operations: [create],
                extra: "nope",
            }
        })
        .expect_err("extra is not a declaration field");
        assert!(err.to_string().contains("extra"));
    }
}
