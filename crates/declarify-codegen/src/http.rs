//! Renders `src/generated/http/<resource>.rs`: `axum` routes and handlers
//! implementing the stable REST contract (`docs/05-rest-client-cli.md`).
//! Generated handlers depend on the generated service trait, not on the
//! default service (`docs/02-architecture.md`); a route/handler is only
//! emitted for an operation the resource actually enables.

use declarify_core::Operation;
use declarify_schema::ResourceSchema;

use crate::util::module_name;
use crate::OWNERSHIP_MARKER;

pub fn render(resource: &ResourceSchema) -> String {
    let name = &resource.name;
    let module = module_name(name);
    let plural = &resource.plural;
    let has = |op: Operation| resource.operations.contains(&op);

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
    out.push_str("use std::sync::Arc;\n\n");
    out.push_str(&format!(
        "use crate::generated::application::{module}::{name}Service;\n\
         use crate::generated::resources::{module}::{{{}}};\n\n",
        resource_imports.join(", ")
    ));

    let base_path = format!("/api/v1/{plural}");
    let id_path = format!("{base_path}/{{id}}");
    let status_path = format!("{id_path}/status");

    out.push_str(&format!(
        "/// Builds the `{base_path}` routes for `{name}`. See docs/05-rest-client-cli.md.\n\
         pub fn routes(service: Arc<dyn {name}Service>) -> axum::Router {{\n\
         let mut router = axum::Router::new();\n"
    ));

    if has(Operation::Create) || has(Operation::List) {
        let mut chain = "axum::routing::MethodRouter::new()".to_string();
        if has(Operation::Create) {
            chain.push_str(".post(create)");
        }
        if has(Operation::List) {
            chain.push_str(".get(list)");
        }
        out.push_str(&format!(
            "router = router.route(\"{base_path}\", {chain});\n"
        ));
    }

    if has(Operation::Get) || has(Operation::UpdateSpec) || has(Operation::Delete) {
        let mut chain = "axum::routing::MethodRouter::new()".to_string();
        if has(Operation::Get) {
            chain.push_str(".get(get)");
        }
        if has(Operation::UpdateSpec) {
            chain.push_str(".put(replace_spec)");
        }
        if has(Operation::Delete) {
            chain.push_str(".delete(delete)");
        }
        out.push_str(&format!("router = router.route(\"{id_path}\", {chain});\n"));
    }

    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "router = router.route(\"{status_path}\", axum::routing::put(update_status));\n"
        ));
    }

    out.push_str("router.with_state(service)\n}\n\n");

    if has(Operation::Create) {
        out.push_str(&format!(
            "async fn create(\n\
             axum::extract::State(service): axum::extract::State<Arc<dyn {name}Service>>,\n\
             axum::Json(input): axum::Json<Create{name}Input>,\n\
             ) -> Result<(axum::http::StatusCode, axum::Json<{name}>), declarify_runtime::http::ApiError> {{\n\
             let created = service.create(input)?;\n\
             Ok((axum::http::StatusCode::CREATED, axum::Json(created)))\n\
             }}\n\n"
        ));
    }

    if has(Operation::Get) {
        out.push_str(&format!(
            "async fn get(\n\
             axum::extract::State(service): axum::extract::State<Arc<dyn {name}Service>>,\n\
             axum::extract::Path(id): axum::extract::Path<String>,\n\
             ) -> Result<axum::Json<{name}>, declarify_runtime::http::ApiError> {{\n\
             Ok(axum::Json(service.get(&id)?))\n\
             }}\n\n"
        ));
    }

    if has(Operation::List) {
        out.push_str(&format!(
            "async fn list(\n\
             axum::extract::State(service): axum::extract::State<Arc<dyn {name}Service>>,\n\
             axum::extract::Query(query): axum::extract::Query<declarify_runtime::http::ListQuery>,\n\
             ) -> Result<axum::Json<declarify_core::Page<{name}>>, declarify_runtime::http::ApiError> {{\n\
             let page = declarify_core::PageRequest {{ limit: query.limit, cursor: query.cursor }};\n\
             Ok(axum::Json(service.list(page)?))\n\
             }}\n\n"
        ));
    }

    if has(Operation::UpdateSpec) {
        out.push_str(&format!(
            "async fn replace_spec(\n\
             axum::extract::State(service): axum::extract::State<Arc<dyn {name}Service>>,\n\
             axum::extract::Path(id): axum::extract::Path<String>,\n\
             headers: axum::http::HeaderMap,\n\
             axum::Json(input): axum::Json<Replace{name}SpecInput>,\n\
             ) -> Result<axum::Json<{name}>, declarify_runtime::http::ApiError> {{\n\
             let expected_revision = declarify_runtime::http::require_if_match(&headers)?;\n\
             Ok(axum::Json(service.replace_spec(&id, expected_revision, input)?))\n\
             }}\n\n"
        ));
    }

    if has(Operation::UpdateStatus) {
        out.push_str(&format!(
            "async fn update_status(\n\
             axum::extract::State(service): axum::extract::State<Arc<dyn {name}Service>>,\n\
             axum::extract::Path(id): axum::extract::Path<String>,\n\
             headers: axum::http::HeaderMap,\n\
             axum::Json(input): axum::Json<Update{name}StatusInput>,\n\
             ) -> Result<axum::Json<{name}>, declarify_runtime::http::ApiError> {{\n\
             let expected_revision = declarify_runtime::http::require_if_match(&headers)?;\n\
             Ok(axum::Json(service.update_status(&id, expected_revision, input)?))\n\
             }}\n\n"
        ));
    }

    if has(Operation::Delete) {
        out.push_str(&format!(
            "async fn delete(\n\
             axum::extract::State(service): axum::extract::State<Arc<dyn {name}Service>>,\n\
             axum::extract::Path(id): axum::extract::Path<String>,\n\
             headers: axum::http::HeaderMap,\n\
             ) -> Result<axum::http::StatusCode, declarify_runtime::http::ApiError> {{\n\
             let expected_revision = declarify_runtime::http::require_if_match(&headers)?;\n\
             service.delete(&id, expected_revision)?;\n\
             Ok(axum::http::StatusCode::NO_CONTENT)\n\
             }}\n\n"
        ));
    }

    out
}
