//! Bambu Lab filament catalogue, loaded-filament import, and project 3MF.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::color::{Color, DEFAULT_MODEL_COLOR};
use crate::document::{Document, Object};
use crate::exchange::{face_normals, to_y_up, to_z_up, unzip, zip_store, ExchangeError};
use crate::mesh::Mesh;
use crate::PLATE_SIZE_MM;

pub const DEFAULT_BAMBU_MATERIAL: &str = "GFA00";
const APP: &str = "BambuStudio-02.00.02.01";
const BED: f32 = PLATE_SIZE_MM;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FilamentRef {
    pub material: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub code: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BambuColor {
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "h")]
    pub hex: String,
    #[serde(rename = "c")]
    pub code: String,
    #[serde(default, rename = "g")]
    pub gradient: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BambuMaterial {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub presets: Vec<String>,
    #[serde(default)]
    pub colors: Vec<BambuColor>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LoadedFilament {
    pub slot: u32,
    pub color: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub material_id: Option<String>,
    #[serde(default)]
    pub color_name: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

fn catalogue() -> &'static [BambuMaterial] {
    static CATALOGUE: std::sync::OnceLock<Vec<BambuMaterial>> = std::sync::OnceLock::new();
    CATALOGUE.get_or_init(|| {
        serde_json::from_str(include_str!("bambu_materials.json"))
            .expect("Bambu filament catalogue")
    })
}

pub fn bambu_materials() -> &'static [BambuMaterial] {
    catalogue()
}

pub fn bambu_material(id: &str) -> &'static BambuMaterial {
    catalogue()
        .iter()
        .find(|material| material.id == id)
        .unwrap_or(&catalogue()[0])
}

pub fn extract_loaded_filaments(
    bytes: &[u8],
    filename: &str,
) -> Result<Vec<LoadedFilament>, ExchangeError> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(ExchangeError::TooLarge);
    }
    let lower = filename.to_ascii_lowercase();
    let is_zip = bytes.len() >= 2 && bytes[0] == 0x50 && bytes[1] == 0x4b;
    let value = if !is_zip {
        serde_json::from_slice::<serde_json::Value>(bytes).map_err(|_| {
            ExchangeError::Bambu(format!("{filename}: not a readable Bambu settings JSON."))
        })?
    } else {
        let files = unzip(bytes)?;
        if lower.ends_with(".3mf") {
            let config = files.iter().find(|(name, _)| {
                name.to_ascii_lowercase()
                    .ends_with("metadata/project_settings.config")
            });
            let Some((_, data)) = config else {
                return Err(ExchangeError::Bambu(format!(
                    "{filename} has no Metadata/project_settings.config: it is not a Bambu Studio project."
                )));
            };
            serde_json::from_slice(data).map_err(|_| {
                ExchangeError::Bambu(format!("{filename}: project settings are not JSON."))
            })?
        } else {
            let mut jsons: Vec<_> = files
                .iter()
                .filter(|(name, _)| {
                    let lower = name.to_ascii_lowercase();
                    lower.ends_with(".json") && !lower.ends_with("bundle_structure.json")
                })
                .collect();
            if jsons.is_empty() {
                return Err(ExchangeError::Bambu(format!(
                    "{filename}: no presets found inside."
                )));
            }
            jsons.sort_by_key(|(name, _)| {
                let lower = name.to_ascii_lowercase();
                if lower.contains("filament") {
                    2
                } else if lower.contains("printer") || lower.contains("machine") {
                    0
                } else {
                    1
                }
            });
            let mut merged = serde_json::Map::new();
            for (name, data) in jsons {
                let value: serde_json::Value = serde_json::from_slice(data).map_err(|_| {
                    ExchangeError::Bambu(format!("{name}: not a readable Bambu settings JSON."))
                })?;
                if let Some(object) = value.as_object() {
                    for (key, item) in object {
                        merged.insert(key.clone(), item.clone());
                    }
                }
            }
            serde_json::Value::Object(merged)
        }
    };
    Ok(filaments_from_value(&value))
}

fn filaments_from_value(value: &serde_json::Value) -> Vec<LoadedFilament> {
    let colours = string_list(
        value
            .get("filament_colour")
            .or_else(|| value.get("default_filament_colour")),
    );
    let types = string_list(value.get("filament_type"));
    let names = string_list(
        value
            .get("filament_settings_id")
            .or_else(|| value.get("name"))
            .or_else(|| value.get("filament_vendor")),
    );
    let ids = string_list(
        value
            .get("filament_ids")
            .or_else(|| value.get("filament_id")),
    );
    let count = colours.len().max(types.len()).max(names.len());
    let mut out = Vec::new();
    for index in 0..count {
        let color = hex_colour(
            colours
                .get(index)
                .or(colours.first())
                .map(String::as_str)
                .unwrap_or(""),
        );
        let color = if color.is_empty() {
            "#cccccc".into()
        } else {
            color
        };
        let kind = types
            .get(index)
            .or(types.first())
            .cloned()
            .unwrap_or_default();
        let raw_name = names
            .get(index)
            .or(names.first())
            .cloned()
            .unwrap_or_else(|| kind.clone());
        let id = ids.get(index).cloned();
        let material = find_material(id.as_deref(), &raw_name);
        let (color_name, code) = material
            .and_then(|item| {
                item.colors
                    .iter()
                    .find(|color_entry| color_entry.hex.eq_ignore_ascii_case(&color))
                    .map(|color_entry| (color_entry.name.clone(), color_entry.code.clone()))
            })
            .map(|(name, code)| (Some(name), Some(code)))
            .unwrap_or((None, None));
        out.push(LoadedFilament {
            slot: index as u32 + 1,
            color,
            kind: if kind.is_empty() {
                material.map(|item| item.kind.clone()).unwrap_or_default()
            } else {
                kind
            },
            name: clean_name(&raw_name)
                .or_else(|| material.map(|item| item.name.clone()))
                .unwrap_or_else(|| "Filament".into()),
            material_id: material.map(|item| item.id.clone()).or(id),
            color_name,
            code,
        });
    }
    out
}

fn string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        None => Vec::new(),
        Some(serde_json::Value::Array(items)) => items.iter().map(value_string).collect(),
        Some(other) => value_string(other)
            .split(';')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect(),
    }
}

fn value_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::Bool(flag) => flag.to_string(),
        _ => String::new(),
    }
}

fn hex_colour(text: &str) -> String {
    let trimmed = text.trim().trim_start_matches('#');
    let hex = trimmed.get(..6).unwrap_or("");
    if hex.len() == 6 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        format!("#{hex}").to_ascii_lowercase()
    } else {
        String::new()
    }
}

fn clean_name(text: &str) -> Option<String> {
    let cleaned = text
        .split('@')
        .next()
        .unwrap_or(text)
        .trim()
        .trim_start_matches("Bambu ")
        .trim_start_matches("bambu ")
        .trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn find_material<'a>(id: Option<&str>, name: &str) -> Option<&'a BambuMaterial> {
    if let Some(id) = id {
        if let Some(material) = catalogue().iter().find(|material| material.id == id) {
            return Some(material);
        }
    }
    let cleaned = clean_name(name).unwrap_or_default().to_ascii_lowercase();
    catalogue()
        .iter()
        .find(|material| material.name.eq_ignore_ascii_case(&cleaned))
}

struct Slot {
    material: String,
    color: String,
    preset: String,
    kind: String,
}

pub fn export_bambu_3mf(doc: &Document, only_selected: bool) -> Result<Vec<u8>, ExchangeError> {
    let selected = doc.selected_ids();
    let bodies: Vec<&Object> = doc
        .objects()
        .iter()
        .filter(|object| doc.is_shown(object.id))
        .filter(|object| !only_selected || selected.is_empty() || selected.contains(&object.id))
        .filter(|object| object.world_mesh().triangle_count() > 0)
        .collect();
    if bodies.is_empty() {
        return Err(ExchangeError::Empty);
    }
    let mut plates: BTreeMap<u32, Vec<&Object>> = BTreeMap::new();
    for object in &bodies {
        let plate = doc.plate_at(object.position);
        plates.entry(plate.number).or_default().push(*object);
    }
    let plate_list: Vec<Vec<&Object>> = plates.into_values().collect();
    let mut slots: Vec<Slot> = Vec::new();
    let mut assignment: Vec<Vec<u32>> = Vec::new();
    for plate in &plate_list {
        let mut row = Vec::new();
        for object in plate {
            let material = object
                .filament
                .as_ref()
                .map(|filament| filament.material.clone())
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| DEFAULT_BAMBU_MATERIAL.into());
            let color = object.color.hex_string().to_ascii_uppercase();
            let key = format!("{material}|{color}");
            let index = slots
                .iter()
                .position(|slot| format!("{}|{}", slot.material, slot.color) == key);
            let index = if let Some(index) = index {
                index
            } else {
                let info = bambu_material(&material);
                let code = if info.presets.iter().any(|preset| preset == "X1C") {
                    "X1C"
                } else {
                    info.presets.first().map(String::as_str).unwrap_or("X1C")
                };
                slots.push(Slot {
                    material: material.clone(),
                    color: color.clone(),
                    preset: format!("Bambu {} @BBL {code}", info.name),
                    kind: info.kind.clone(),
                });
                slots.len() - 1
            };
            row.push(index as u32 + 1);
        }
        assignment.push(row);
    }
    if slots.is_empty() {
        let info = bambu_material(DEFAULT_BAMBU_MATERIAL);
        slots.push(Slot {
            material: info.id.clone(),
            color: DEFAULT_MODEL_COLOR.hex_string(),
            preset: format!("Bambu {} @BBL X1C", info.name),
            kind: info.kind.clone(),
        });
    }

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut wrappers = String::new();
    let mut items = String::new();
    let mut rels = String::new();
    let mut object_settings = String::new();
    let mut plate_settings = String::new();
    let mut assemble = String::new();
    let count = plate_list.len().max(1);
    let mut object_number = 0u32;
    let mut triangles = 0usize;
    for (plate_index, plate) in plate_list.iter().enumerate() {
        let origin = bambu_origin(plate_index, count);
        let mut instances = String::new();
        for (body_index, object) in plate.iter().enumerate() {
            let mesh = object.world_mesh();
            triangles += mesh.triangle_count();
            object_number += 1;
            let screen = doc.plate_at(object.position);
            let (verts, transform) = bambu_mesh(&mesh, screen.center_x, screen.center_z, origin);
            let path = format!("3D/Objects/object_{object_number}.model");
            let model = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n<object id=\"1\" type=\"model\"><mesh><vertices>{verts}</vertices><triangles>{}</triangles></mesh></object>\n</model>\n",
                mesh.indices.chunks(3).map(|tri| format!("<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", tri[0], tri[1], tri[2])).collect::<String>()
            );
            files.push((path.clone(), model.into_bytes()));
            let wrapper = 1000 + object_number;
            wrappers.push_str(&format!(
                "<object id=\"{wrapper}\" type=\"model\"><components><component path=\"/{path}\" objectid=\"1\"/></components></object>\n"
            ));
            items.push_str(&format!(
                "<item objectid=\"{wrapper}\" transform=\"{transform}\" printable=\"1\"/>\n"
            ));
            rels.push_str(&format!(
                "<Relationship Target=\"/{path}\" Id=\"rel-{object_number}\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>\n"
            ));
            let extruder = assignment[plate_index][body_index];
            let name = xml_escape(&object.name);
            object_settings.push_str(&format!(
                "<object id=\"{wrapper}\"><metadata key=\"name\" value=\"{name}\"/><metadata key=\"extruder\" value=\"{extruder}\"/></object>\n"
            ));
            instances.push_str(&format!(
                "<model_instance><metadata key=\"object_id\" value=\"{wrapper}\"/><metadata key=\"instance_id\" value=\"0\"/></model_instance>\n"
            ));
            assemble.push_str(&format!(
                "<assemble_item object_id=\"{wrapper}\" instance_id=\"0\" transform=\"{transform}\"/>\n"
            ));
        }
        plate_settings.push_str(&format!(
            "<plate><metadata key=\"plater_id\" value=\"{}\"/><metadata key=\"plater_name\" value=\"PrintCaddy plate {}\"/>{instances}</plate>\n",
            plate_index + 1,
            plate_index + 1
        ));
    }
    crate::exchange::guard_tris(triangles)?;
    let model = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n<metadata name=\"Application\">{APP}</metadata>\n<resources>\n{wrappers}</resources>\n<build>\n{items}</build>\n</model>\n"
    );
    let settings = project_settings(&slots);
    let model_settings = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n{object_settings}{plate_settings}<assemble>\n{assemble}</assemble>\n</config>\n"
    );
    let mut packed: Vec<(String, Vec<u8>)> = files;
    packed.push(("[Content_Types].xml".into(), CONTENT_TYPES.into()));
    packed.push(("_rels/.rels".into(), ROOT_RELS.into()));
    packed.push(("3D/3dmodel.model".into(), model.into_bytes()));
    packed.push((
        "3D/_rels/3dmodel.model.rels".into(),
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n{rels}</Relationships>\n").into_bytes(),
    ));
    packed.push((
        "Metadata/model_settings.config".into(),
        model_settings.into_bytes(),
    ));
    packed.push((
        "Metadata/project_settings.config".into(),
        settings.into_bytes(),
    ));
    packed.push((
        "Metadata/slice_info.config".into(),
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config><header><header_item key=\"X-BBL-Client-Type\" value=\"slicer\"/></header></config>\n".to_vec(),
    ));
    let refs: Vec<(&str, &[u8])> = packed
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();
    Ok(zip_store(&refs))
}

fn project_settings(slots: &[Slot]) -> String {
    let printer = "Bambu Lab X1 Carbon 0.4 nozzle";
    let process = "0.20mm Standard @BBL X1C";
    let mut inherits = vec![process.to_string()];
    inherits.extend(slots.iter().map(|slot| slot.preset.clone()));
    inherits.push(printer.to_string());
    let mut different = vec!["print_settings_id".to_string()];
    different.extend(slots.iter().map(|_| "filament_colour".to_string()));
    different.push("printer_settings_id".to_string());
    let value = serde_json::json!({
        "from": "project",
        "name": "project_settings",
        "version": "02.00.02.01",
        "printer_model": "Bambu Lab X1 Carbon",
        "printer_settings_id": printer,
        "printer_variant": "0.4",
        "print_settings_id": process,
        "filament_settings_id": slots.iter().map(|slot| &slot.preset).collect::<Vec<_>>(),
        "filament_type": slots.iter().map(|slot| &slot.kind).collect::<Vec<_>>(),
        "filament_ids": slots.iter().map(|slot| &slot.material).collect::<Vec<_>>(),
        "filament_colour": slots.iter().map(|slot| &slot.color).collect::<Vec<_>>(),
        "filament_vendor": slots.iter().map(|_| "Bambu Lab").collect::<Vec<_>>(),
        "inherits_group": inherits,
        "different_settings_to_system": different,
        "printable_area": ["0x0", format!("{BED}x0"), format!("{BED}x{BED}"), format!("0x{BED}")],
        "printable_height": "256",
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

fn bambu_origin(index: usize, count: usize) -> (f32, f32) {
    let value = (count as f32).sqrt();
    let rounded = value.round();
    let cols = if value > rounded {
        rounded + 1.0
    } else {
        rounded
    };
    let cols = cols.max(1.0) as usize;
    let x = (index % cols) as f32 * BED * 1.2;
    let y = -((index / cols) as f32) * BED * 1.2;
    (x, y)
}

fn bambu_mesh(mesh: &Mesh, plate_x: f32, plate_z: f32, origin: (f32, f32)) -> (String, String) {
    let mut points = Vec::new();
    for position in &mesh.positions {
        let file = to_z_up(*position);
        points.push([
            file[0] - plate_x + BED * 0.5,
            file[1] + plate_z + BED * 0.5,
            file[2],
        ]);
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y, mut min_z) =
        (f32::MAX, f32::MIN, f32::MAX, f32::MIN, f32::MAX);
    for point in &points {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
        min_z = min_z.min(point[2]);
    }
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let mut verts = String::new();
    for point in &points {
        verts.push_str(&format!(
            "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
            num(point[0] - cx),
            num(point[1] - cy),
            num(point[2] - min_z)
        ));
    }
    let transform = format!(
        "1 0 0 0 1 0 0 0 1 {} {} {}",
        num(origin.0 + cx),
        num(origin.1 + cy),
        num(min_z.max(0.0))
    );
    (verts, transform)
}

pub fn import_bambu_3mf(doc: &mut Document, bytes: &[u8]) -> Result<(), ExchangeError> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(ExchangeError::TooLarge);
    }
    let files = unzip(bytes)?;
    let settings = files
        .iter()
        .find(|(name, _)| {
            name.to_ascii_lowercase()
                .ends_with("metadata/project_settings.config")
        })
        .map(|(_, data)| data.as_slice())
        .ok_or_else(|| {
            ExchangeError::Bambu(
                "That 3MF has no Metadata/project_settings.config, so it is not a Bambu project."
                    .into(),
            )
        })?;
    let settings: serde_json::Value = serde_json::from_slice(settings)
        .map_err(|_| ExchangeError::Bambu("Bambu project settings are not JSON.".into()))?;
    let colours = string_list(settings.get("filament_colour"));
    let ids = string_list(settings.get("filament_ids"));
    let model_settings = files
        .iter()
        .find(|(name, _)| {
            name.to_ascii_lowercase()
                .ends_with("metadata/model_settings.config")
        })
        .map(|(_, data)| String::from_utf8_lossy(data).to_string())
        .unwrap_or_default();
    let wrapper = files
        .iter()
        .find(|(name, _)| name.to_ascii_lowercase().ends_with("3d/3dmodel.model"))
        .map(|(_, data)| String::from_utf8_lossy(data).to_string())
        .ok_or(ExchangeError::Malformed)?;
    let mut added = 0;
    for object in xml_blocks(&wrapper, "object") {
        let Some(id) = attr(&object, "id") else {
            continue;
        };
        let Some(path) = attr(&object, "path").or_else(|| component_path(&object)) else {
            continue;
        };
        let file = files.iter().find(|(name, _)| {
            name.trim_start_matches('/')
                .eq_ignore_ascii_case(path.trim_start_matches('/'))
        });
        let Some((_, data)) = file else { continue };
        let text = String::from_utf8_lossy(data);
        let Some(mesh) = mesh_from_model(&text)? else {
            continue;
        };
        let transform = item_transform(&wrapper, &id).unwrap_or([0.0, 0.0, 0.0]);
        let mut positions = Vec::new();
        for position in mesh.positions {
            positions.push(to_y_up([
                position[0] + transform[0],
                position[1] + transform[1],
                position[2] + transform[2],
            ]));
        }
        let normals = face_normals(&positions, &mesh.indices);
        let extruder = object_extruder(&model_settings, &id).unwrap_or(1);
        let color = colours
            .get(extruder.saturating_sub(1) as usize)
            .and_then(|text| parse_colour(text))
            .unwrap_or(DEFAULT_MODEL_COLOR);
        let name = object_name(&model_settings, &id).unwrap_or_else(|| format!("Bambu {extruder}"));
        let placed = doc.add_placed_mesh(
            name,
            color,
            Mesh {
                positions,
                normals,
                indices: mesh.indices,
            },
        );
        if let Some(material) = ids.get(extruder.saturating_sub(1) as usize) {
            doc.set_filament(
                placed,
                FilamentRef {
                    material: material.clone(),
                    name: String::new(),
                    code: String::new(),
                },
            );
        }
        added += 1;
    }
    if added == 0 {
        return Err(ExchangeError::Malformed);
    }
    Ok(())
}

struct LocalMesh {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn mesh_from_model(text: &str) -> Result<Option<LocalMesh>, ExchangeError> {
    let mut positions = Vec::new();
    for tag in crate::exchange::xml_tags(text, "vertex") {
        positions.push([
            crate::exchange::attr_f32(&tag, "x")?,
            crate::exchange::attr_f32(&tag, "y")?,
            crate::exchange::attr_f32(&tag, "z")?,
        ]);
    }
    let mut indices = Vec::new();
    for tag in crate::exchange::xml_tags(text, "triangle") {
        indices.push(crate::exchange::attr_u32(&tag, "v1")?);
        indices.push(crate::exchange::attr_u32(&tag, "v2")?);
        indices.push(crate::exchange::attr_u32(&tag, "v3")?);
    }
    if positions.is_empty() || indices.is_empty() {
        return Ok(None);
    }
    Ok(Some(LocalMesh { positions, indices }))
}

fn xml_blocks<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let open = format!("<{name} ");
    let close = format!("</{name}>");
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start..];
        let Some(end) = after.find(&close) else { break };
        out.push(&after[..end]);
        rest = &after[end + close.len()..];
    }
    out
}

fn component_path(block: &str) -> Option<String> {
    attr(block, "path")
}

fn attr(tag: &str, name: &str) -> Option<String> {
    crate::exchange::attr(tag, name)
}

fn item_transform(model: &str, object_id: &str) -> Option<[f32; 3]> {
    for tag in crate::exchange::xml_tags(model, "item") {
        if attr(&tag, "objectid").as_deref() == Some(object_id) {
            let transform = attr(&tag, "transform")?;
            let nums: Vec<f32> = transform
                .split_whitespace()
                .filter_map(|part| part.parse().ok())
                .collect();
            if nums.len() >= 12 {
                return Some([nums[9], nums[10], nums[11]]);
            }
        }
    }
    None
}

fn object_extruder(settings: &str, id: &str) -> Option<u32> {
    let needle = format!("id=\"{id}\"");
    let start = settings.find(&needle)?;
    let end = settings[start..].find("</object>").unwrap_or(400) + start;
    let block = &settings[start..end];
    for tag in crate::exchange::xml_tags(block, "metadata") {
        if attr(&tag, "key").as_deref() == Some("extruder") {
            return attr(&tag, "value").and_then(|value| value.parse().ok());
        }
    }
    None
}

fn object_name(settings: &str, id: &str) -> Option<String> {
    let needle = format!("id=\"{id}\"");
    let start = settings.find(&needle)?;
    let end = settings[start..].find("</object>").unwrap_or(400) + start;
    let block = &settings[start..end];
    for tag in crate::exchange::xml_tags(block, "metadata") {
        if attr(&tag, "key").as_deref() == Some("name") {
            return attr(&tag, "value");
        }
    }
    None
}

fn parse_colour(text: &str) -> Option<Color> {
    let hex = hex_colour(text);
    if hex.is_empty() {
        None
    } else {
        u32::from_str_radix(hex.trim_start_matches('#'), 16)
            .ok()
            .map(Color::from_hex)
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn num(value: f32) -> String {
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    if rounded == 0.0 {
        "0".into()
    } else {
        format!("{rounded}")
    }
}

const CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
 <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
 <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>"#;

const ROOT_RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/3D/3dmodel.model" Id="rel-1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>"#;
