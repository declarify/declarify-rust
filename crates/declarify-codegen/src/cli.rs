//! Renders `src/generated/cli/<resource>.rs`: `clap` subcommands calling the
//! generated typed client (`docs/05-rest-client-cli.md`, "CLI"). A generated
//! command exists only for an operation the resource actually enables.
//!
//! Command contract (recorded in `docs/09-decisions.md`): `create` reads a
//! full `Create<Name>Input` JSON document from stdin (it alone carries
//! `name` alongside `spec`); `update`/`status` read the bare `Spec`/`Status`
//! JSON document from stdin and take `--expected-revision` as a flag rather
//! than parsing it back out of a full resource document; `delete` takes only
//! `--expected-revision`. Every command prints its JSON result to stdout and
//! propagates failures as `Err`, so the handwritten binary entry point can
//! print to stderr and exit non-zero, exactly as `docs/05-rest-client-cli.md`
//! specifies.

use declarify_core::Operation;
use declarify_schema::ResourceSchema;

use crate::util::{domain_import, module_name};
use crate::OWNERSHIP_MARKER;

pub fn render(resource: &ResourceSchema) -> String {
    let name = &resource.name;
    let module = module_name(name);
    let has = |op: Operation| resource.operations.contains(&op);

    let mut resource_imports = Vec::new();
    if has(Operation::Create) {
        resource_imports.push(format!("Create{name}Input"));
    }
    if has(Operation::UpdateSpec) {
        resource_imports.push(format!("Replace{name}SpecInput"));
    }
    if has(Operation::UpdateStatus) {
        resource_imports.push(format!("Update{name}StatusInput"));
    }
    resource_imports.sort();

    let mut domain_imports = Vec::new();
    if has(Operation::UpdateSpec) {
        domain_imports.push(domain_import(&resource.spec));
    }
    if has(Operation::UpdateStatus) {
        domain_imports.push(domain_import(&resource.status));
    }
    domain_imports.sort();
    domain_imports.dedup();

    let mut out = String::new();
    out.push_str(OWNERSHIP_MARKER);
    out.push('\n');
    for import in &domain_imports {
        out.push_str(import);
        out.push('\n');
    }
    out.push_str(&format!(
        "use crate::generated::client::{module}::{name}Client;\n"
    ));
    if !resource_imports.is_empty() {
        out.push_str(&format!(
            "use crate::generated::resources::{module}::{{{}}};\n",
            resource_imports.join(", ")
        ));
    }
    out.push('\n');

    // --- Command enum ----------------------------------------------------
    out.push_str(&format!(
        "/// `{name}` subcommands. See docs/05-rest-client-cli.md.\n\
         #[derive(clap::Subcommand)]\n\
         pub enum {name}Command {{\n"
    ));
    if has(Operation::Create) {
        out.push_str(&format!(
            "/// Create a `{name}`. Reads a `Create{name}Input` JSON document from stdin.\n\
             Create,\n"
        ));
    }
    if has(Operation::Get) {
        out.push_str(&format!(
            "/// Get a `{name}` by id.\n\
             Get {{ id: String }},\n"
        ));
    }
    if has(Operation::List) {
        out.push_str(
            "/// List `{name}`s.\n\
             List {\n\
             #[arg(long)]\n\
             limit: Option<u32>,\n\
             #[arg(long)]\n\
             cursor: Option<String>,\n\
             },\n"
                .replace("{name}", name)
                .as_str(),
        );
    }
    if has(Operation::UpdateSpec) {
        out.push_str(&format!(
            "/// Replace a `{name}`'s spec. Reads a `{spec}` JSON document from stdin.\n\
             Update {{\n\
             id: String,\n\
             #[arg(long)]\n\
             expected_revision: u64,\n\
             }},\n",
            spec = resource.spec.name,
        ));
    }
    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "/// Update a `{name}`'s status. Reads a `{status}` JSON document from stdin.\n\
             Status {{\n\
             id: String,\n\
             #[arg(long)]\n\
             expected_revision: u64,\n\
             }},\n",
            status = resource.status.name,
        ));
    }
    if has(Operation::Delete) {
        out.push_str(
            "/// Delete a `{name}`.\n\
             Delete {\n\
             id: String,\n\
             #[arg(long)]\n\
             expected_revision: u64,\n\
             },\n"
                .replace("{name}", name)
                .as_str(),
        );
    }
    out.push_str("}\n\n");

    // --- Dispatch ----------------------------------------------------------
    out.push_str(&format!(
        "/// Runs `command` against `client`, printing its JSON result to stdout.\n\
         pub async fn run(\n\
         command: {name}Command,\n\
         client: &{name}Client,\n\
         ) -> Result<(), declarify_runtime::cli::CliError> {{\n\
         match command {{\n"
    ));

    if has(Operation::Create) {
        out.push_str(&format!(
            "{name}Command::Create => {{\n\
             let input: Create{name}Input = declarify_runtime::cli::read_json_stdin()?;\n\
             let created = client.create(input).await?;\n\
             declarify_runtime::cli::print_json(&created)?;\n\
             }}\n"
        ));
    }
    if has(Operation::Get) {
        out.push_str(&format!(
            "{name}Command::Get {{ id }} => {{\n\
             let resource = client.get(&id).await?;\n\
             declarify_runtime::cli::print_json(&resource)?;\n\
             }}\n"
        ));
    }
    if has(Operation::List) {
        out.push_str(&format!(
            "{name}Command::List {{ limit, cursor }} => {{\n\
             let page = client\n\
             .list(declarify_core::PageRequest {{ limit, cursor }})\n\
             .await?;\n\
             declarify_runtime::cli::print_json(&page)?;\n\
             }}\n"
        ));
    }
    if has(Operation::UpdateSpec) {
        out.push_str(&format!(
            "{name}Command::Update {{ id, expected_revision }} => {{\n\
             let spec: {spec} = declarify_runtime::cli::read_json_stdin()?;\n\
             let updated = client\n\
             .replace_spec(&id, expected_revision, Replace{name}SpecInput {{ spec }})\n\
             .await?;\n\
             declarify_runtime::cli::print_json(&updated)?;\n\
             }}\n",
            spec = resource.spec.name,
        ));
    }
    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "{name}Command::Status {{ id, expected_revision }} => {{\n\
             let status: {status} = declarify_runtime::cli::read_json_stdin()?;\n\
             let updated = client\n\
             .update_status(&id, expected_revision, Update{name}StatusInput {{ status }})\n\
             .await?;\n\
             declarify_runtime::cli::print_json(&updated)?;\n\
             }}\n",
            status = resource.status.name,
        ));
    }
    if has(Operation::Delete) {
        out.push_str(&format!(
            "{name}Command::Delete {{ id, expected_revision }} => {{\n\
             client.delete(&id, expected_revision).await?;\n\
             }}\n"
        ));
    }

    out.push_str("}\nOk(())\n}\n");

    out
}
