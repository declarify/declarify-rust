//! Renders `src/generated/application/<resource>.rs`: the repository trait,
//! the validator seam, the service trait, and the default CRUD service. See
//! `docs/06-extension-model.md` for the seams this must preserve and
//! `docs/02-architecture.md` for the dependency direction (default services
//! depend on repository traits, not SQL implementations).

use declarify_core::Operation;
use declarify_schema::ResourceSchema;

use crate::util::{domain_import, module_name};
use crate::OWNERSHIP_MARKER;

pub fn render(resource: &ResourceSchema) -> String {
    let name = &resource.name;
    let spec = &resource.spec.name;
    let status = &resource.status.name;
    let module = module_name(name);
    let has = |op: Operation| resource.operations.contains(&op);

    let mut imports = vec![
        domain_import(&resource.spec),
        domain_import(&resource.status),
    ];
    imports.sort();
    imports.dedup();

    let mut resource_imports = vec![name.clone()];
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

    let mut out = String::new();
    out.push_str(OWNERSHIP_MARKER);
    out.push('\n');
    for import in &imports {
        out.push_str(import);
        out.push('\n');
    }
    out.push_str(&format!(
        "use crate::generated::resources::{module}::{{{}}};\n\n",
        resource_imports.join(", ")
    ));

    // --- Repository trait -------------------------------------------------
    out.push_str(&format!(
        "/// Persistence port for `{name}`. Implemented by a generated SQL \
         adapter or any handwritten repository, e.g. a test double.\n\
         pub trait {name}Repository: Send + Sync {{\n"
    ));
    if has(Operation::Create) {
        out.push_str(&format!(
            "fn insert(&self, resource: {name}) -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::Get) {
        out.push_str(&format!(
            "fn get(&self, id: &str) -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::List) {
        out.push_str(&format!(
            "fn list(&self, page: declarify_core::PageRequest) -> \
             Result<declarify_core::Page<{name}>, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::UpdateSpec) {
        out.push_str(&format!(
            "fn replace_spec(&self, id: &str, expected_revision: u64, spec: {spec}, \
             updated_at: chrono::DateTime<chrono::Utc>) -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "fn update_status(&self, id: &str, expected_revision: u64, status: {status}, \
             updated_at: chrono::DateTime<chrono::Utc>) -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::Delete) {
        out.push_str(
            "fn delete(&self, id: &str, expected_revision: u64) -> Result<(), declarify_core::Error>;\n",
        );
    }
    out.push_str("}\n\n");

    // --- Validator seam -----------------------------------------------------
    let validates_spec = has(Operation::Create) || has(Operation::UpdateSpec);
    let validates_status = has(Operation::UpdateStatus);
    out.push_str(&format!(
        "/// Domain validation seam for `{name}`. The default implementation \
         accepts everything; replace it via composition.\n\
         pub trait {name}Validator: Send + Sync {{\n"
    ));
    if validates_spec {
        out.push_str(&format!(
            "fn validate_spec(&self, _spec: &{spec}) -> Result<(), declarify_core::Error> {{ Ok(()) }}\n"
        ));
    }
    if validates_status {
        out.push_str(&format!(
            "fn validate_status(&self, _status: &{status}) -> Result<(), declarify_core::Error> {{ Ok(()) }}\n"
        ));
    }
    out.push_str("}\n\n");
    out.push_str(&format!(
        "/// The no-op default `{name}Validator`.\n\
         #[derive(Debug, Clone, Copy, Default)]\n\
         pub struct NoOp{name}Validator;\n\n\
         impl {name}Validator for NoOp{name}Validator {{}}\n\n"
    ));

    // --- Service trait --------------------------------------------------
    out.push_str(&format!(
        "/// Business-facing operations for `{name}`. Generated HTTP handlers \
         depend on this trait, not on `Default{name}Service` directly.\n\
         pub trait {name}Service: Send + Sync {{\n"
    ));
    if has(Operation::Create) {
        out.push_str(&format!(
            "fn create(&self, input: Create{name}Input) -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::Get) {
        out.push_str(&format!(
            "fn get(&self, id: &str) -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::List) {
        out.push_str(&format!(
            "fn list(&self, page: declarify_core::PageRequest) -> \
             Result<declarify_core::Page<{name}>, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::UpdateSpec) {
        out.push_str(&format!(
            "fn replace_spec(&self, id: &str, expected_revision: u64, input: Replace{name}SpecInput) \
             -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "fn update_status(&self, id: &str, expected_revision: u64, input: Update{name}StatusInput) \
             -> Result<{name}, declarify_core::Error>;\n"
        ));
    }
    if has(Operation::Delete) {
        out.push_str(
            "fn delete(&self, id: &str, expected_revision: u64) -> Result<(), declarify_core::Error>;\n",
        );
    }
    out.push_str("}\n\n");

    // --- Default service --------------------------------------------------
    out.push_str(&format!(
        "/// Default `{name}Service`, generic over an injected repository, clock, \
         ID generator, and validator. Replace any of these via composition; \
         replace the whole service by implementing `{name}Service` directly.\n\
         pub struct Default{name}Service<\n\
         R,\n\
         C = declarify_runtime::SystemClock,\n\
         I = declarify_runtime::UuidGenerator,\n\
         V = NoOp{name}Validator,\n\
         > {{\n\
         repository: R,\n\
         clock: C,\n\
         id_generator: I,\n\
         validator: V,\n\
         }}\n\n\
         impl<R, C, I, V> Default{name}Service<R, C, I, V> {{\n\
         pub fn new(repository: R, clock: C, id_generator: I, validator: V) -> Self {{\n\
         Self {{ repository, clock, id_generator, validator }}\n\
         }}\n\
         }}\n\n"
    ));

    out.push_str(&format!(
        "impl<R, C, I, V> {name}Service for Default{name}Service<R, C, I, V>\n\
         where\n\
         R: {name}Repository,\n\
         C: declarify_runtime::Clock,\n\
         I: declarify_runtime::IdGenerator,\n\
         V: {name}Validator,\n\
         {{\n"
    ));

    if has(Operation::Create) {
        out.push_str(&format!(
            "fn create(&self, input: Create{name}Input) -> Result<{name}, declarify_core::Error> {{\n\
             self.validator.validate_spec(&input.spec)?;\n\
             let now = self.clock.now();\n\
             let resource = {name} {{\n\
             type_meta: declarify_core::TypeMeta {{\n\
             kind: \"{name}\".to_string(),\n\
             api_version: \"{api_version}\".to_string(),\n\
             }},\n\
             metadata: declarify_core::Metadata {{\n\
             id: self.id_generator.generate(),\n\
             name: input.name,\n\
             labels: Default::default(),\n\
             annotations: Default::default(),\n\
             revision: 1,\n\
             created_at: now,\n\
             updated_at: now,\n\
             }},\n\
             spec: input.spec,\n\
             status: {status}::default(),\n\
             }};\n\
             self.repository.insert(resource)\n\
             }}\n\n",
            api_version = resource.api_version,
        ));
    }

    if has(Operation::Get) {
        out.push_str(&format!(
            "fn get(&self, id: &str) -> Result<{name}, declarify_core::Error> {{\n\
             self.repository.get(id)\n\
             }}\n\n"
        ));
    }

    if has(Operation::List) {
        out.push_str(&format!(
            "fn list(&self, page: declarify_core::PageRequest) -> \
             Result<declarify_core::Page<{name}>, declarify_core::Error> {{\n\
             self.repository.list(page)\n\
             }}\n\n"
        ));
    }

    if has(Operation::UpdateSpec) {
        out.push_str(&format!(
            "fn replace_spec(&self, id: &str, expected_revision: u64, input: Replace{name}SpecInput) \
             -> Result<{name}, declarify_core::Error> {{\n\
             self.validator.validate_spec(&input.spec)?;\n\
             let now = self.clock.now();\n\
             self.repository.replace_spec(id, expected_revision, input.spec, now)\n\
             }}\n\n"
        ));
    }

    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "fn update_status(&self, id: &str, expected_revision: u64, input: Update{name}StatusInput) \
             -> Result<{name}, declarify_core::Error> {{\n\
             self.validator.validate_status(&input.status)?;\n\
             let now = self.clock.now();\n\
             self.repository.update_status(id, expected_revision, input.status, now)\n\
             }}\n\n"
        ));
    }

    if has(Operation::Delete) {
        out.push_str(
            "fn delete(&self, id: &str, expected_revision: u64) -> Result<(), declarify_core::Error> {\n\
             self.repository.delete(id, expected_revision)\n\
             }\n\n",
        );
    }

    out.push_str("}\n");

    out
}
