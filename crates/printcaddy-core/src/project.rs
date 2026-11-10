//! A saved design. The file holds the bodies. It does not hold the undo stack.

use serde::{Deserialize, Serialize};

use crate::document::{Document, Object};

pub const PROJECT_FORMAT: &str = "printcaddy-project";
pub const PROJECT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectError {
    NotAProject,
    UnsupportedVersion,
    BadJson,
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            ProjectError::NotAProject => "That file is not a PrintCaddy project.",
            ProjectError::UnsupportedVersion => "That project was saved by a newer PrintCaddy.",
            ProjectError::BadJson => "That project file could not be read.",
        };
        f.write_str(text)
    }
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    format: String,
    version: u32,
    objects: Vec<Object>,
    next_id: u64,
    #[serde(default)]
    plates: Vec<u32>,
    #[serde(default)]
    locked_plates: Vec<u32>,
}

pub fn document_to_json(doc: &Document) -> String {
    let file = ProjectFile {
        format: PROJECT_FORMAT.into(),
        version: PROJECT_VERSION,
        objects: doc.objects().to_vec(),
        next_id: doc.next_id_value(),
        plates: doc.extra_slots().to_vec(),
        locked_plates: doc.locked_slots().to_vec(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".into())
}

pub fn document_from_json(text: &str) -> Result<Document, ProjectError> {
    let file: ProjectFile = serde_json::from_str(text).map_err(|_| ProjectError::BadJson)?;
    if file.format != PROJECT_FORMAT {
        return Err(ProjectError::NotAProject);
    }
    if file.version == 0 || file.version > PROJECT_VERSION {
        return Err(ProjectError::UnsupportedVersion);
    }
    let mut doc = Document::from_objects(file.objects, file.next_id);
    doc.restore_plates(file.plates, file.locked_plates);
    Ok(doc)
}
