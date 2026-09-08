use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSpec {
    pub title: String,
    pub priority: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NoteStatus {
    pub indexed: bool,
}

declarify::resource! {
    pub Note {
        api_version: "notes.declarify.dev/v1",
        plural: "notes",
        spec: NoteSpec,
        status: NoteStatus,
        operations: [create, get, list, update_spec, update_status, delete],
    }
}
