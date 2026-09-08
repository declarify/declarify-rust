//! Positive/negative fixtures for the DCL-0001 scanner, per
//! `docs/07-scenarios-and-quality.md` scenarios 7 and 8.

use std::path::{Path, PathBuf};

use declarify_core::Operation;
use declarify_schema::{scan_source_root, FieldType};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/declarify-schema")
        .join(name)
}

#[test]
fn valid_notes_fixture_resolves_the_declared_resource() {
    let outcome = scan_source_root(&fixture("valid/notes"));
    assert!(
        outcome.diagnostics.is_empty(),
        "unexpected diagnostics: {:#?}",
        outcome.diagnostics
    );
    assert_eq!(outcome.resources.len(), 1);

    let note = &outcome.resources[0];
    assert_eq!(note.name, "Note");
    assert_eq!(note.api_version, "notes.declarify.dev/v1");
    assert_eq!(note.plural, "notes");
    assert_eq!(
        note.operations,
        vec![
            Operation::Create,
            Operation::Get,
            Operation::List,
            Operation::UpdateSpec,
            Operation::UpdateStatus,
            Operation::Delete,
        ]
    );

    assert_eq!(note.spec.name, "NoteSpec");
    assert_eq!(note.spec.module_path, "note");
    assert_eq!(note.spec.fields.len(), 2);
    assert_eq!(note.spec.fields[0].name, "title");
    assert_eq!(note.spec.fields[0].ty, FieldType::String);
    assert_eq!(note.spec.fields[1].name, "body");
    assert_eq!(note.spec.fields[1].ty, FieldType::String);

    assert_eq!(note.status.name, "NoteStatus");
    assert_eq!(note.status.fields.len(), 1);
    assert_eq!(note.status.fields[0].name, "indexed");
    assert_eq!(note.status.fields[0].ty, FieldType::Bool);
}

#[test]
fn missing_referenced_spec_type_is_a_diagnostic_not_a_panic() {
    let outcome = scan_source_root(&fixture("invalid/missing_spec_type"));
    assert!(outcome.resources.is_empty());
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("NoteSpec") && d.message.contains("not found")),
        "diagnostics: {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn duplicate_resource_name_is_rejected() {
    let outcome = scan_source_root(&fixture("invalid/duplicate_resource"));
    assert_eq!(outcome.resources.len(), 1, "first declaration still wins");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate resource name `Note`")),
        "diagnostics: {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn unsupported_field_type_is_rejected_with_field_name() {
    let outcome = scan_source_root(&fixture("invalid/unsupported_field_type"));
    assert!(outcome.resources.is_empty());
    assert!(
        outcome.diagnostics.iter().any(|d| d.message.contains("priority")
            && d.message.contains("unsupported field type")),
        "diagnostics: {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn status_type_without_default_is_rejected() {
    let outcome = scan_source_root(&fixture("invalid/status_missing_default"));
    assert!(outcome.resources.is_empty());
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("NoteStatus") && d.message.contains("Default")),
        "diagnostics: {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn unsupported_serde_field_attribute_is_rejected() {
    let outcome = scan_source_root(&fixture("invalid/unsupported_serde_attr"));
    assert!(outcome.resources.is_empty());
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("title") && d.message.contains("serde")),
        "diagnostics: {:#?}",
        outcome.diagnostics
    );
}
