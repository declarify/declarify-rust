//! Renders `src/generated/resources/<resource>.rs`: the concrete resource
//! alias and the request/input types for its enabled mutating operations.
//! See `docs/01-resource-model.md` and `docs/03-declaration-and-generation.md`
//! ("Generated contract").

use declarify_core::Operation;
use declarify_schema::ResourceSchema;

use crate::util::domain_import;
use crate::OWNERSHIP_MARKER;

pub fn render(resource: &ResourceSchema) -> String {
    let mut imports = vec![
        domain_import(&resource.spec),
        domain_import(&resource.status),
    ];
    imports.sort();
    imports.dedup();

    let mut out = String::new();
    out.push_str(OWNERSHIP_MARKER);
    out.push('\n');
    for import in &imports {
        out.push_str(import);
        out.push('\n');
    }
    out.push('\n');

    out.push_str(&format!(
        "/// The `{name}` resource (`{api_version}`).\n\
         pub type {name} = declarify_core::Resource<{spec}, {status}>;\n\n",
        name = resource.name,
        api_version = resource.api_version,
        spec = resource.spec.name,
        status = resource.status.name,
    ));

    if resource.operations.contains(&Operation::Create) {
        out.push_str(&format!(
            "/// Input for creating a `{name}`.\n\
             #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n\
             pub struct Create{name}Input {{\n\
             pub name: Option<String>,\n\
             pub spec: {spec},\n\
             }}\n\n",
            name = resource.name,
            spec = resource.spec.name,
        ));
    }

    if resource.operations.contains(&Operation::UpdateSpec) {
        out.push_str(&format!(
            "/// Input for replacing a `{name}`'s spec.\n\
             #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n\
             pub struct Replace{name}SpecInput {{\n\
             pub spec: {spec},\n\
             }}\n\n",
            name = resource.name,
            spec = resource.spec.name,
        ));
    }

    if resource.operations.contains(&Operation::UpdateStatus) {
        out.push_str(&format!(
            "/// Input for updating a `{name}`'s status.\n\
             #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n\
             pub struct Update{name}StatusInput {{\n\
             pub status: {status},\n\
             }}\n\n",
            name = resource.name,
            status = resource.status.name,
        ));
    }

    out
}
