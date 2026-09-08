//! Renders `src/generated/persistence/<resource>.rs`: fresh-schema SQL, a
//! `migrate` function, and a `rusqlite`-backed repository implementing the
//! resource's generated repository trait. See `docs/04-storage.md` for the
//! product rules this must preserve: one table per resource, `spec_`/
//! `status_` column prefixes, and a mutation is only ever atomic through the
//! SQL predicate/transaction — connection creation and backend selection
//! stay handwritten.

use declarify_core::Operation;
use declarify_schema::{FieldSchema, FieldType, ResourceSchema};

use crate::util::{domain_import, module_name};
use crate::OWNERSHIP_MARKER;

struct Column {
    name: String,
    sql_type: &'static str,
}

fn envelope_columns() -> Vec<Column> {
    vec![
        Column {
            name: "id".to_string(),
            sql_type: "TEXT PRIMARY KEY",
        },
        Column {
            name: "name".to_string(),
            sql_type: "TEXT",
        },
        Column {
            name: "labels".to_string(),
            sql_type: "TEXT NOT NULL",
        },
        Column {
            name: "annotations".to_string(),
            sql_type: "TEXT NOT NULL",
        },
        Column {
            name: "revision".to_string(),
            sql_type: "INTEGER NOT NULL",
        },
        Column {
            name: "created_at".to_string(),
            sql_type: "TEXT NOT NULL",
        },
        Column {
            name: "updated_at".to_string(),
            sql_type: "TEXT NOT NULL",
        },
    ]
}

fn field_columns(prefix: &str, fields: &[FieldSchema]) -> Vec<Column> {
    fields
        .iter()
        .map(|f| Column {
            name: format!("{prefix}_{}", f.name),
            sql_type: match f.ty {
                FieldType::String => "TEXT NOT NULL",
                FieldType::Bool => "INTEGER NOT NULL",
            },
        })
        .collect()
}

fn all_columns(resource: &ResourceSchema) -> Vec<Column> {
    let mut columns = envelope_columns();
    columns.extend(field_columns("spec", &resource.spec.fields));
    columns.extend(field_columns("status", &resource.status.fields));
    columns
}

pub fn render(resource: &ResourceSchema) -> String {
    let name = &resource.name;
    let table = &resource.plural;
    let module = module_name(name);
    let columns = all_columns(resource);
    let column_list = columns
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let mut imports = vec![
        domain_import(&resource.spec),
        domain_import(&resource.status),
    ];
    imports.sort();
    imports.dedup();

    let mut out = String::new();
    out.push_str(OWNERSHIP_MARKER);
    out.push('\n');
    out.push_str("use std::sync::Mutex;\n\n");
    for import in &imports {
        out.push_str(import);
        out.push('\n');
    }
    out.push_str(&format!(
        "use crate::generated::application::{module}::{name}Repository;\n\
         use crate::generated::resources::{module}::{name};\n\n"
    ));

    // --- Schema/migration --------------------------------------------------
    let column_defs = columns
        .iter()
        .map(|c| format!("    {} {}", c.name, c.sql_type))
        .collect::<Vec<_>>()
        .join(",\n");
    out.push_str(&format!(
        "/// Fresh-schema SQL for the `{table}` table. Column names use `spec_`/\n\
         /// `status_` prefixes (docs/04-storage.md). General migration between\n\
         /// schema versions is out of scope for Step 0.\n\
         pub const SCHEMA_SQL: &str = \"CREATE TABLE IF NOT EXISTS {table} (\\n{}\\n)\";\n\n\
         /// Creates the `{table}` table if it does not already exist.\n\
         pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {{\n\
         conn.execute_batch(SCHEMA_SQL)\n\
         }}\n\n",
        column_defs.replace('\n', "\\n")
    ));

    // --- Row mapping --------------------------------------------------------
    out.push_str(&format!(
        "fn row_to_{module}(row: &rusqlite::Row<'_>) -> rusqlite::Result<{name}> {{\n\
         let labels_json: String = row.get(2)?;\n\
         let annotations_json: String = row.get(3)?;\n\
         let revision: i64 = row.get(4)?;\n\
         Ok({name} {{\n\
         type_meta: declarify_core::TypeMeta {{\n\
         kind: \"{name}\".to_string(),\n\
         api_version: \"{api_version}\".to_string(),\n\
         }},\n\
         metadata: declarify_core::Metadata {{\n\
         id: row.get(0)?,\n\
         name: row.get(1)?,\n\
         labels: serde_json::from_str(&labels_json).unwrap_or_default(),\n\
         annotations: serde_json::from_str(&annotations_json).unwrap_or_default(),\n\
         revision: revision as u64,\n\
         created_at: row.get(5)?,\n\
         updated_at: row.get(6)?,\n\
         }},\n\
         spec: {spec}{{\n{spec_fields}}},\n\
         status: {status}{{\n{status_fields}}},\n\
         }})\n\
         }}\n\n\
         fn internal_err(err: impl std::fmt::Display) -> declarify_core::Error {{\n\
         declarify_core::Error::Internal(err.to_string())\n\
         }}\n\n",
        api_version = resource.api_version,
        spec = resource.spec.name,
        status = resource.status.name,
        spec_fields = row_field_assignments(&resource.spec.fields, "spec", 7),
        status_fields = row_field_assignments(
            &resource.status.fields,
            "status",
            7 + resource.spec.fields.len()
        ),
    ));

    // --- Repository ----------------------------------------------------
    out.push_str(&format!(
        "/// SQLite-backed `{name}Repository`. See docs/04-storage.md: connection\n\
         /// creation and backend selection remain handwritten composition.\n\
         pub struct Sqlite{name}Repository {{\n\
         conn: Mutex<rusqlite::Connection>,\n\
         }}\n\n\
         impl Sqlite{name}Repository {{\n\
         pub fn new(conn: rusqlite::Connection) -> Self {{\n\
         Self {{ conn: Mutex::new(conn) }}\n\
         }}\n\
         }}\n\n\
         fn lock_poisoned() -> declarify_core::Error {{\n\
         declarify_core::Error::Internal(\"poisoned SQLite connection mutex\".to_string())\n\
         }}\n\n\
         fn existence_error(\n\
         tx: &rusqlite::Transaction<'_>,\n\
         id: &str,\n\
         ) -> Result<declarify_core::Error, declarify_core::Error> {{\n\
         let exists: bool = tx\n\
         .query_row(\n\
         \"SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)\",\n\
         rusqlite::params![id],\n\
         |row| row.get(0),\n\
         )\n\
         .map_err(internal_err)?;\n\
         Ok(if exists {{ declarify_core::Error::Conflict }} else {{ declarify_core::Error::NotFound }})\n\
         }}\n\n"
    ));

    out.push_str(&format!(
        "impl {name}Repository for Sqlite{name}Repository {{\n"
    ));

    if resource.operations.contains(&Operation::Create) {
        let insert_placeholders = (1..=columns.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let insert_values = insert_value_exprs(resource);
        out.push_str(&format!(
            "fn insert(&self, resource: {name}) -> Result<{name}, declarify_core::Error> {{\n\
             let conn = self.conn.lock().map_err(|_| lock_poisoned())?;\n\
             let labels_json = serde_json::to_string(&resource.metadata.labels).map_err(internal_err)?;\n\
             let annotations_json = serde_json::to_string(&resource.metadata.annotations).map_err(internal_err)?;\n\
             conn.execute(\n\
             \"INSERT INTO {table} ({column_list}) VALUES ({insert_placeholders})\",\n\
             rusqlite::params![{insert_values}],\n\
             )\n\
             .map_err(internal_err)?;\n\
             Ok(resource)\n\
             }}\n\n"
        ));
    }

    if resource.operations.contains(&Operation::Get) {
        out.push_str(&format!(
            "fn get(&self, id: &str) -> Result<{name}, declarify_core::Error> {{\n\
             let conn = self.conn.lock().map_err(|_| lock_poisoned())?;\n\
             conn.query_row(\n\
             \"SELECT {column_list} FROM {table} WHERE id = ?1\",\n\
             rusqlite::params![id],\n\
             row_to_{module},\n\
             )\n\
             .map_err(|err| match err {{\n\
             rusqlite::Error::QueryReturnedNoRows => declarify_core::Error::NotFound,\n\
             other => internal_err(other),\n\
             }})\n\
             }}\n\n"
        ));
    }

    if resource.operations.contains(&Operation::List) {
        out.push_str(&format!(
            "fn list(&self, page: declarify_core::PageRequest) -> \
             Result<declarify_core::Page<{name}>, declarify_core::Error> {{\n\
             let limit = page.limit.unwrap_or(50).max(1) as i64;\n\
             let conn = self.conn.lock().map_err(|_| lock_poisoned())?;\n\
             let mut stmt = conn\n\
             .prepare(\"SELECT {column_list} FROM {table} WHERE (?1 IS NULL OR id > ?1) ORDER BY id ASC LIMIT ?2\")\n\
             .map_err(internal_err)?;\n\
             let rows = stmt\n\
             .query_map(rusqlite::params![page.cursor, limit + 1], row_to_{module})\n\
             .map_err(internal_err)?;\n\
             let mut items = Vec::new();\n\
             for row in rows {{\n\
             items.push(row.map_err(internal_err)?);\n\
             }}\n\
             let next_cursor = if items.len() as i64 > limit {{\n\
             items.truncate(limit as usize);\n\
             items.last().map(|item| item.metadata.id.clone())\n\
             }} else {{\n\
             None\n\
             }};\n\
             Ok(declarify_core::Page {{ items, next_cursor }})\n\
             }}\n\n"
        ));
    }

    if resource.operations.contains(&Operation::UpdateSpec) {
        let assignments = resource
            .spec
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("spec_{} = ?{}", f.name, i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let n = resource.spec.fields.len();
        let updated_at_pos = n + 1;
        let id_pos = n + 2;
        let revision_pos = n + 3;
        let field_values = resource
            .spec
            .fields
            .iter()
            .map(|f| format!("spec.{}", f.name))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "fn replace_spec(\n\
             &self,\n\
             id: &str,\n\
             expected_revision: u64,\n\
             spec: {spec},\n\
             updated_at: chrono::DateTime<chrono::Utc>,\n\
             ) -> Result<{name}, declarify_core::Error> {{\n\
             let mut conn = self.conn.lock().map_err(|_| lock_poisoned())?;\n\
             let tx = conn.transaction().map_err(internal_err)?;\n\
             let rows_affected = tx\n\
             .execute(\n\
             \"UPDATE {table} SET {assignments}, revision = revision + 1, updated_at = ?{updated_at_pos} \
             WHERE id = ?{id_pos} AND revision = ?{revision_pos}\",\n\
             rusqlite::params![{field_values}, updated_at, id, expected_revision as i64],\n\
             )\n\
             .map_err(internal_err)?;\n\
             if rows_affected == 0 {{\n\
             return Err(existence_error(&tx, id)?);\n\
             }}\n\
             let updated = tx\n\
             .query_row(\"SELECT {column_list} FROM {table} WHERE id = ?1\", rusqlite::params![id], row_to_{module})\n\
             .map_err(internal_err)?;\n\
             tx.commit().map_err(internal_err)?;\n\
             Ok(updated)\n\
             }}\n\n",
            spec = resource.spec.name,
        ));
    }

    if resource.operations.contains(&Operation::UpdateStatus) {
        let assignments = resource
            .status
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("status_{} = ?{}", f.name, i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let n = resource.status.fields.len();
        let updated_at_pos = n + 1;
        let id_pos = n + 2;
        let revision_pos = n + 3;
        let field_values = resource
            .status
            .fields
            .iter()
            .map(|f| format!("status.{}", f.name))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "fn update_status(\n\
             &self,\n\
             id: &str,\n\
             expected_revision: u64,\n\
             status: {status},\n\
             updated_at: chrono::DateTime<chrono::Utc>,\n\
             ) -> Result<{name}, declarify_core::Error> {{\n\
             let mut conn = self.conn.lock().map_err(|_| lock_poisoned())?;\n\
             let tx = conn.transaction().map_err(internal_err)?;\n\
             let rows_affected = tx\n\
             .execute(\n\
             \"UPDATE {table} SET {assignments}, revision = revision + 1, updated_at = ?{updated_at_pos} \
             WHERE id = ?{id_pos} AND revision = ?{revision_pos}\",\n\
             rusqlite::params![{field_values}, updated_at, id, expected_revision as i64],\n\
             )\n\
             .map_err(internal_err)?;\n\
             if rows_affected == 0 {{\n\
             return Err(existence_error(&tx, id)?);\n\
             }}\n\
             let updated = tx\n\
             .query_row(\"SELECT {column_list} FROM {table} WHERE id = ?1\", rusqlite::params![id], row_to_{module})\n\
             .map_err(internal_err)?;\n\
             tx.commit().map_err(internal_err)?;\n\
             Ok(updated)\n\
             }}\n\n",
            status = resource.status.name,
        ));
    }

    if resource.operations.contains(&Operation::Delete) {
        out.push_str(&format!(
            "fn delete(&self, id: &str, expected_revision: u64) -> Result<(), declarify_core::Error> {{\n\
             let mut conn = self.conn.lock().map_err(|_| lock_poisoned())?;\n\
             let tx = conn.transaction().map_err(internal_err)?;\n\
             let rows_affected = tx\n\
             .execute(\n\
             \"DELETE FROM {table} WHERE id = ?1 AND revision = ?2\",\n\
             rusqlite::params![id, expected_revision as i64],\n\
             )\n\
             .map_err(internal_err)?;\n\
             if rows_affected == 0 {{\n\
             return Err(existence_error(&tx, id)?);\n\
             }}\n\
             tx.commit().map_err(internal_err)?;\n\
             Ok(())\n\
             }}\n\n"
        ));
    }

    out.push_str("}\n");

    out
}

fn row_field_assignments(fields: &[FieldSchema], _prefix: &str, start_index: usize) -> String {
    fields
        .iter()
        .enumerate()
        .map(|(i, f)| format!("{}: row.get({})?,\n", f.name, start_index + i))
        .collect::<Vec<_>>()
        .join("")
}

fn insert_value_exprs(resource: &ResourceSchema) -> String {
    let mut values = vec![
        "resource.metadata.id".to_string(),
        "resource.metadata.name".to_string(),
        "labels_json".to_string(),
        "annotations_json".to_string(),
        "resource.metadata.revision as i64".to_string(),
        "resource.metadata.created_at".to_string(),
        "resource.metadata.updated_at".to_string(),
    ];
    for f in &resource.spec.fields {
        values.push(format!("resource.spec.{}", f.name));
    }
    for f in &resource.status.fields {
        values.push(format!("resource.status.{}", f.name));
    }
    values.join(", ")
}
