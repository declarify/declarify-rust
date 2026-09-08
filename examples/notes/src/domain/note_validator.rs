use crate::domain::note::NoteSpec;
use crate::generated::application::note::NoteValidator;

/// Rejects a `Note` whose title is blank. Demonstrates the validator seam
/// from `docs/06-extension-model.md` ("domain validation") composed with the
/// generated default service without editing any generated file.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonEmptyTitleValidator;

impl NoteValidator for NonEmptyTitleValidator {
    fn validate_spec(&self, spec: &NoteSpec) -> Result<(), declarify_core::Error> {
        if spec.title.trim().is_empty() {
            return Err(declarify_core::Error::InvalidArgument(
                "title must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}
