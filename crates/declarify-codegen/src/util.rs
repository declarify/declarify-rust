use declarify_schema::TypeSchema;

/// Converts a `PascalCase` resource name into its `snake_case` module/file
/// name, e.g. `Note` -> `note`, `NoteItem` -> `note_item`.
pub fn module_name(resource_name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in resource_name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Renders a `use` statement importing `ty` from the application's domain
/// module, using the module path recorded by the scanner.
pub fn domain_import(ty: &TypeSchema) -> String {
    if ty.module_path.is_empty() {
        format!("use crate::{name};", name = ty.name)
    } else {
        format!(
            "use crate::{path}::{name};",
            path = ty.module_path,
            name = ty.name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name_converts_pascal_case() {
        assert_eq!(module_name("Note"), "note");
        assert_eq!(module_name("NoteItem"), "note_item");
    }
}
