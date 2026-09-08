//! Parses the body of a `resource!` macro invocation found in source text.
//!
//! This mirrors the grammar accepted by `declarify-macros`, but a proc-macro
//! crate can only export macros, not reusable parsing code, so the scanner
//! keeps its own copy driven by the same fixed grammar and the same
//! [`declarify_core::Operation`] vocabulary. `docs/09-decisions.md` records
//! this as a deliberate, tested duplication rather than a shared dependency.

use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, LitStr, Token, Visibility,
};

pub struct RawOperation {
    pub keyword: String,
    pub span: proc_macro2::Span,
}

pub struct RawResourceDecl {
    pub name: String,
    pub name_span: proc_macro2::Span,
    pub api_version: String,
    pub plural: String,
    pub spec: String,
    pub spec_span: proc_macro2::Span,
    pub status: String,
    pub status_span: proc_macro2::Span,
    pub operations: Vec<RawOperation>,
}

enum Field {
    ApiVersion(LitStr),
    Plural(LitStr),
    Spec(Ident),
    Status(Ident),
    Operations(Vec<Ident>),
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

impl Parse for RawResourceDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _vis: Visibility = input.parse()?;
        let name: Ident = input.parse()?;

        let body;
        braced!(body in input);
        let fields: Punctuated<Field, Token![,]> =
            body.parse_terminated(Field::parse, Token![,])?;

        let mut api_version = None;
        let mut plural = None;
        let mut spec: Option<Ident> = None;
        let mut status: Option<Ident> = None;
        let mut operations: Option<Vec<Ident>> = None;

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
        if operations.is_none() {
            missing.push("operations");
        }
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

        let spec = spec.unwrap();
        let status = status.unwrap();
        let operations = operations
            .unwrap()
            .into_iter()
            .map(|ident| RawOperation {
                keyword: ident.to_string(),
                span: ident.span(),
            })
            .collect::<Vec<_>>();

        if operations.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                format!("resource `{name}` must enable at least one operation"),
            ));
        }

        Ok(RawResourceDecl {
            name: name.to_string(),
            name_span: name.span(),
            api_version: api_version.unwrap().value(),
            plural: plural.unwrap().value(),
            spec: spec.to_string(),
            spec_span: spec.span(),
            status: status.to_string(),
            status_span: status.span(),
            operations,
        })
    }
}

/// Parses the token stream inside `resource! { .. }` into a [`RawResourceDecl`].
pub fn parse_resource_macro(tokens: proc_macro2::TokenStream) -> syn::Result<RawResourceDecl> {
    syn::parse2(tokens)
}

/// Returns `true` when `path`'s last segment is `resource`, i.e. the macro
/// was invoked as `resource!(..)` or `declarify::resource!(..)` (or through
/// any re-export ending in that name).
pub fn is_resource_macro_path(path: &syn::Path) -> bool {
    path.segments
        .last()
        .map(|seg| seg.ident == "resource")
        .unwrap_or(false)
}
