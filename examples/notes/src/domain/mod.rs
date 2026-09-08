// `pub` is required: generated code imports Spec/Status types by their exact
// file-based module path (`crate::domain::note::NoteSpec`), not through this
// re-export. See docs/09-decisions.md.
pub mod note;
pub mod note_validator;

pub use note::{NoteSpec, NoteStatus};
pub use note_validator::NonEmptyTitleValidator;
