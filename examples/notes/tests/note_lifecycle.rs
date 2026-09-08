//! Exercises the full `Note` lifecycle through generated application code and
//! handwritten composition, backed by an in-memory repository test double —
//! no SQL/REST/CLI involved (SQLite is covered separately by
//! `note_lifecycle_sqlite.rs`; REST/CLI land in DCL-0004/0005). This is
//! scenario 10 of `docs/07-scenarios-and-quality.md`: "a handwritten
//! validator, service, repository test double, ... compose with generated
//! code without generated edits."

mod support;

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use notes::composition::build_note_service;
use notes::domain::{NoteSpec, NoteStatus};
use notes::generated::application::note::NoteRepository;
use notes::generated::resources::note::Note;

/// The "repository test double" scenario 10 calls for, standing in for the
/// real SQLite adapter.
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
fn full_lifecycle_through_in_memory_repository() {
    let service = build_note_service(InMemoryNoteRepository::default());
    support::run_full_lifecycle(service);
}
