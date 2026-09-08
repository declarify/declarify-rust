//! Exercises the full `Note` lifecycle through generated application code and
//! handwritten composition, with no SQL/REST/CLI involved yet (those land in
//! DCL-0003/0004/0005). This is scenario 10 of
//! `docs/07-scenarios-and-quality.md`: "a handwritten validator, service,
//! repository test double, ... compose with generated code without generated
//! edits."

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use notes::composition::build_note_service;
use notes::domain::{NoteSpec, NoteStatus};
use notes::generated::application::note::{NoteRepository, NoteService};
use notes::generated::resources::note::{
    CreateNoteInput, Note, ReplaceNoteSpecInput, UpdateNoteStatusInput,
};

/// The "repository test double" scenario 10 calls for, standing in for the
/// real SQLite adapter until DCL-0003.
#[derive(Default)]
struct InMemoryNoteRepository {
    notes: Mutex<HashMap<String, Note>>,
}

impl NoteRepository for InMemoryNoteRepository {
    fn insert(&self, resource: Note) -> Result<Note, declarify_core::Error> {
        self.notes
            .lock()
            .unwrap()
            .insert(resource.metadata.id.clone(), resource.clone());
        Ok(resource)
    }

    fn get(&self, id: &str) -> Result<Note, declarify_core::Error> {
        self.notes
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or(declarify_core::Error::NotFound)
    }

    fn list(
        &self,
        _page: declarify_core::PageRequest,
    ) -> Result<declarify_core::Page<Note>, declarify_core::Error> {
        let notes = self.notes.lock().unwrap();
        let mut items: Vec<Note> = notes.values().cloned().collect();
        items.sort_by(|a, b| a.metadata.id.cmp(&b.metadata.id));
        Ok(declarify_core::Page {
            items,
            next_cursor: None,
        })
    }

    fn replace_spec(
        &self,
        id: &str,
        expected_revision: u64,
        spec: NoteSpec,
        updated_at: DateTime<Utc>,
    ) -> Result<Note, declarify_core::Error> {
        let mut notes = self.notes.lock().unwrap();
        let note = notes.get_mut(id).ok_or(declarify_core::Error::NotFound)?;
        if note.metadata.revision != expected_revision {
            return Err(declarify_core::Error::Conflict);
        }
        note.spec = spec;
        note.metadata.revision += 1;
        note.metadata.updated_at = updated_at;
        Ok(note.clone())
    }

    fn update_status(
        &self,
        id: &str,
        expected_revision: u64,
        status: NoteStatus,
        updated_at: DateTime<Utc>,
    ) -> Result<Note, declarify_core::Error> {
        let mut notes = self.notes.lock().unwrap();
        let note = notes.get_mut(id).ok_or(declarify_core::Error::NotFound)?;
        if note.metadata.revision != expected_revision {
            return Err(declarify_core::Error::Conflict);
        }
        note.status = status;
        note.metadata.revision += 1;
        note.metadata.updated_at = updated_at;
        Ok(note.clone())
    }

    fn delete(&self, id: &str, expected_revision: u64) -> Result<(), declarify_core::Error> {
        let mut notes = self.notes.lock().unwrap();
        let note = notes.get(id).ok_or(declarify_core::Error::NotFound)?;
        if note.metadata.revision != expected_revision {
            return Err(declarify_core::Error::Conflict);
        }
        notes.remove(id);
        Ok(())
    }
}

#[test]
fn full_lifecycle_through_generated_service_and_handwritten_composition() {
    let service = build_note_service(InMemoryNoteRepository::default());

    let created = service
        .create(CreateNoteInput {
            name: Some("first".to_string()),
            spec: NoteSpec {
                title: "Title".into(),
                body: "Body".into(),
            },
        })
        .expect("create succeeds");
    assert_eq!(created.metadata.revision, 1);
    assert!(!created.status.indexed);

    let fetched = service.get(&created.metadata.id).expect("get succeeds");
    assert_eq!(fetched.spec.title, "Title");

    let listed = service
        .list(declarify_core::PageRequest::default())
        .expect("list succeeds");
    assert_eq!(listed.items.len(), 1);

    let replaced = service
        .replace_spec(
            &created.metadata.id,
            created.metadata.revision,
            ReplaceNoteSpecInput {
                spec: NoteSpec {
                    title: "New title".into(),
                    body: "Body".into(),
                },
            },
        )
        .expect("replace_spec succeeds");
    assert_eq!(replaced.metadata.revision, 2);
    assert_eq!(replaced.spec.title, "New title");
    assert!(
        !replaced.status.indexed,
        "replace_spec must preserve status"
    );

    let stale = service.replace_spec(
        &created.metadata.id,
        1, // the pre-replace revision, now stale
        ReplaceNoteSpecInput {
            spec: NoteSpec {
                title: "Stale write".into(),
                body: "Body".into(),
            },
        },
    );
    assert_eq!(stale.unwrap_err(), declarify_core::Error::Conflict);

    let status_updated = service
        .update_status(
            &created.metadata.id,
            replaced.metadata.revision,
            UpdateNoteStatusInput {
                status: NoteStatus { indexed: true },
            },
        )
        .expect("update_status succeeds");
    assert_eq!(status_updated.metadata.revision, 3);
    assert_eq!(
        status_updated.spec.title, "New title",
        "update_status must preserve spec"
    );
    assert!(status_updated.status.indexed);

    let rejected = service.create(CreateNoteInput {
        name: None,
        spec: NoteSpec {
            title: "   ".into(),
            body: String::new(),
        },
    });
    assert!(
        matches!(rejected, Err(declarify_core::Error::InvalidArgument(_))),
        "the handwritten NonEmptyTitleValidator must reject a blank title"
    );

    service
        .delete(&created.metadata.id, status_updated.metadata.revision)
        .expect("delete succeeds");
    assert_eq!(
        service.get(&created.metadata.id).unwrap_err(),
        declarify_core::Error::NotFound
    );
}
