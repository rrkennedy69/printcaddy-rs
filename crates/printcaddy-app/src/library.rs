//! Designs kept in this browser. The undo stack is not stored with them.

use printcaddy_core::{
    document_from_json, document_to_json, Document, LoadedFilament, ProjectError,
};

#[derive(Clone, Debug)]
pub struct SavedProject {
    pub id: u64,
    pub name: String,
    pub body: String,
}

const KEY: &str = "printcaddy.projects.v1";
const LOADED: &str = "printcaddy.loaded.v1";

pub fn load_filaments() -> Vec<LoadedFilament> {
    read_key(LOADED)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn store_filaments(filaments: &[LoadedFilament]) {
    if let Ok(text) = serde_json::to_string(filaments) {
        write_key(LOADED, &text);
    }
}
const SETTINGS: &str = "printcaddy.settings.v1";

pub fn load() -> Vec<SavedProject> {
    let Some(raw) = read_key(KEY) else {
        return Vec::new();
    };
    parse_library(&raw)
}

pub fn store(projects: &[SavedProject]) {
    write_key(KEY, &format_library(projects));
}

pub fn load_settings() -> (String, bool) {
    let Some(raw) = read_key(SETTINGS) else {
        return ("Light".into(), false);
    };
    let theme = field(&raw, "theme").unwrap_or_else(|| "Light".into());
    let mobile = raw.contains("\"mobile\":true");
    (theme, mobile)
}

pub fn store_settings(theme: &str, mobile: bool) {
    write_key(
        SETTINGS,
        &format!(
            "{{\"theme\":\"{theme}\",\"mobile\":{}}}",
            if mobile { "true" } else { "false" }
        ),
    );
}

pub fn save_current(projects: &mut Vec<SavedProject>, name: &str, doc: &Document) -> u64 {
    let body = document_to_json(doc);
    let id = projects.iter().map(|project| project.id).max().unwrap_or(0) + 1;
    projects.push(SavedProject {
        id,
        name: name.to_string(),
        body,
    });
    store(projects);
    id
}

pub fn open_saved(projects: &[SavedProject], id: u64) -> Result<Document, ProjectError> {
    let project = projects
        .iter()
        .find(|project| project.id == id)
        .ok_or(ProjectError::NotAProject)?;
    document_from_json(&project.body)
}

fn format_library(projects: &[SavedProject]) -> String {
    let mut out = String::from("[");
    for (index, project) in projects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"id\":{},\"name\":{},\"body\":{}}}",
            project.id,
            serde_json_string(&project.name),
            serde_json_string(&project.body)
        ));
    }
    out.push(']');
    out
}

fn parse_library(raw: &str) -> Vec<SavedProject> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            Some(SavedProject {
                id: item.get("id")?.as_u64()?,
                name: item.get("name")?.as_str()?.to_string(),
                body: item.get("body")?.as_str()?.to_string(),
            })
        })
        .collect()
}

fn field(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = raw.find(&needle)? + needle.len();
    let end = raw[start..].find('"')? + start;
    Some(raw[start..end].to_string())
}

fn serde_json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

fn read_key(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()?.local_storage().ok()??;
        storage.get_item(key).ok()?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

fn write_key(key: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.set_item(key, value);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, value);
    }
}
