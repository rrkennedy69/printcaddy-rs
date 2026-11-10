//! 3MF, OBJ, SVG, DXF, and a triangulated STEP subset.
//!
//! Files are millimetres and Z-up. The document is Y-up: file `(x, y, z)` is plate `(x, z, −y)`.

use std::io::Read;

use glam::Vec3;

use crate::color::{Color, DEFAULT_MODEL_COLOR};
use crate::document::{Document, ObjectId};
use crate::mesh::Mesh;
use crate::sketch::{SketchDef, SketchError, SketchOp};

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_TRIANGLES: usize = 200_000;
const MAX_OUTLINE_POINTS: usize = 8_000;
const SVG_DXF_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExchangeError {
    Empty,
    TooLarge,
    TooManyTriangles,
    Malformed,
    UnsupportedStep,
    Outline(String),
    Bambu(String),
}

impl std::fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            ExchangeError::Empty => "There is nothing to export.",
            ExchangeError::TooLarge => "That file is over the size limit (32 MB, or 2 MB for SVG and DXF).",
            ExchangeError::TooManyTriangles => "That file has more than 200000 triangles.",
            ExchangeError::Malformed => "That file could not be read.",
            ExchangeError::UnsupportedStep => "This STEP file has no triangulated faces. This build reads POLY_LOOP faces of CARTESIAN_POINT coordinates, in millimetres, Z-up. Exact B-rep is not read.",
            ExchangeError::Outline(text) => text.as_str(),
            ExchangeError::Bambu(text) => text.as_str(),
        };
        f.write_str(text)
    }
}

impl From<SketchError> for ExchangeError {
    fn from(err: SketchError) -> Self {
        ExchangeError::Outline(err.to_string())
    }
}

struct Body {
    name: String,
    color: Color,
    mesh: Mesh,
}

pub(crate) fn to_z_up(p: [f32; 3]) -> [f32; 3] {
    [p[0], -p[2], p[1]]
}

pub(crate) fn to_y_up(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[2], -p[1]]
}

fn guard_bytes(bytes: &[u8], limit: usize) -> Result<(), ExchangeError> {
    if bytes.len() > limit {
        Err(ExchangeError::TooLarge)
    } else {
        Ok(())
    }
}

pub(crate) fn guard_tris(count: usize) -> Result<(), ExchangeError> {
    if count > MAX_TRIANGLES {
        Err(ExchangeError::TooManyTriangles)
    } else {
        Ok(())
    }
}

fn shown_bodies(doc: &Document, only_selected: bool) -> Vec<Body> {
    let selected = doc.selected_ids();
    doc.objects()
        .iter()
        .filter(|object| doc.is_shown(object.id))
        .filter(|object| !only_selected || selected.is_empty() || selected.contains(&object.id))
        .map(|object| Body {
            name: object.name.clone(),
            color: object.color,
            mesh: object.world_mesh(),
        })
        .filter(|body| body.mesh.triangle_count() > 0)
        .collect()
}

fn place(doc: &mut Document, name: &str, color: Color, mesh: Mesh) -> ObjectId {
    doc.add_placed_mesh(name, color, mesh)
}

pub fn export_3mf(doc: &Document, only_selected: bool) -> Result<Vec<u8>, ExchangeError> {
    let bodies = shown_bodies(doc, only_selected);
    if bodies.is_empty() {
        return Err(ExchangeError::Empty);
    }
    let mut tris = 0;
    for body in &bodies {
        tris += body.mesh.triangle_count();
    }
    guard_tris(tris)?;
    let model = model_xml(&bodies);
    Ok(zip_store(&[
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes()),
        ("_rels/.rels", ROOT_RELS.as_bytes()),
        ("3D/3dmodel.model", model.as_bytes()),
    ]))
}

pub fn import_3mf(doc: &mut Document, bytes: &[u8]) -> Result<(), ExchangeError> {
    guard_bytes(bytes, MAX_BYTES)?;
    let files = unzip(bytes)?;
    let model = files
        .iter()
        .find(|(name, _)| name.to_ascii_lowercase().ends_with(".model"))
        .map(|(_, data)| data)
        .ok_or(ExchangeError::Malformed)?;
    let text = String::from_utf8_lossy(model);
    let bodies = parse_3mf(&text)?;
    if bodies.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    for body in bodies {
        place(doc, &body.name, body.color, body.mesh);
    }
    Ok(())
}

pub fn document_to_obj(doc: &Document, only_selected: bool) -> String {
    let bodies = shown_bodies(doc, only_selected);
    let mut lines = vec![
        "# PrintCaddy OBJ export".to_string(),
        "# Z-up, millimetres".to_string(),
    ];
    let mut offset = 1usize;
    for body in bodies {
        lines.push(format!("o {}", body.name.replace('\n', " ")));
        let color = obj_color(body.color);
        for position in &body.mesh.positions {
            let p = to_z_up(*position);
            lines.push(format!(
                "v {} {} {} {}",
                num(p[0]),
                num(p[1]),
                num(p[2]),
                color
            ));
        }
        for tri in body.mesh.indices.chunks_exact(3) {
            lines.push(format!(
                "f {} {} {}",
                tri[0] as usize + offset,
                tri[1] as usize + offset,
                tri[2] as usize + offset
            ));
        }
        offset += body.mesh.positions.len();
    }
    lines.join("\n")
}

pub fn document_from_obj(doc: &mut Document, bytes: &[u8]) -> Result<(), ExchangeError> {
    guard_bytes(bytes, MAX_BYTES)?;
    let text = String::from_utf8_lossy(bytes);
    let bodies = parse_obj(&text)?;
    if bodies.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    for body in bodies {
        place(doc, &body.name, body.color, body.mesh);
    }
    Ok(())
}

pub fn import_svg(doc: &mut Document, bytes: &[u8]) -> Result<(), ExchangeError> {
    guard_bytes(bytes, SVG_DXF_BYTES)?;
    let text = String::from_utf8_lossy(bytes);
    let outlines = svg_outlines(&text)?;
    add_outlines(doc, "SVG", &outlines)
}

pub fn import_dxf(doc: &mut Document, bytes: &[u8]) -> Result<(), ExchangeError> {
    guard_bytes(bytes, SVG_DXF_BYTES)?;
    let text = String::from_utf8_lossy(bytes);
    let outlines = dxf_outlines(&text)?;
    add_outlines(doc, "DXF", &outlines)
}

pub fn document_to_step(doc: &Document, only_selected: bool) -> Result<String, ExchangeError> {
    let bodies = shown_bodies(doc, only_selected);
    if bodies.is_empty() {
        return Err(ExchangeError::Empty);
    }
    let mut tris = 0;
    for body in &bodies {
        tris += body.mesh.triangle_count();
    }
    guard_tris(tris)?;
    let mut id = 2u32;
    let mut data = String::from("#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n");
    for body in &bodies {
        for tri in body.mesh.indices.chunks_exact(3) {
            let mut refs = Vec::new();
            for index in tri {
                let p = to_z_up(body.mesh.positions[*index as usize]);
                data.push_str(&format!(
                    "#{id}=CARTESIAN_POINT('',({:.6},{:.6},{:.6}));\n",
                    p[0], p[1], p[2]
                ));
                refs.push(format!("#{id}"));
                id += 1;
            }
            data.push_str(&format!("#{id}=POLY_LOOP('',({}));\n", refs.join(",")));
            id += 1;
        }
    }
    Ok(format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('PrintCaddy triangulated solid'),'2;1');\nFILE_NAME('printcaddy.step','2026-11-10T00:00:00',('PrintCaddy'),(''),'PrintCaddy','PrintCaddy','');\nFILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));\nENDSEC;\nDATA;\n{data}ENDSEC;\nEND-ISO-10303-21;\n"
    ))
}

pub fn document_from_step(doc: &mut Document, bytes: &[u8]) -> Result<(), ExchangeError> {
    guard_bytes(bytes, MAX_BYTES)?;
    let text = String::from_utf8_lossy(bytes);
    if !text.contains("ISO-10303-21") {
        return Err(ExchangeError::Malformed);
    }
    let mesh = step_mesh(&text)?;
    place(doc, "STEP", DEFAULT_MODEL_COLOR, mesh);
    Ok(())
}

fn add_outlines(
    doc: &mut Document,
    name: &str,
    outlines: &[Vec<[f32; 2]>],
) -> Result<(), ExchangeError> {
    if outlines.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    for (index, outline) in outlines.iter().enumerate() {
        if outline.len() > MAX_OUTLINE_POINTS {
            return Err(ExchangeError::TooManyTriangles);
        }
        let label = if outlines.len() == 1 {
            name.to_string()
        } else {
            format!("{name} {}", index + 1)
        };
        let def = SketchDef::on_plate(SketchOp::Extrude, outline.clone(), vec![], 10.0, 360.0);
        let id = doc.add_sketch(def)?;
        if let Some(object) = doc.object_mut(id) {
            object.name = label;
        }
    }
    Ok(())
}

fn model_xml(bodies: &[Body]) -> String {
    let mut materials = String::new();
    let mut objects = String::new();
    let mut build = String::new();
    for (index, body) in bodies.iter().enumerate() {
        let mat = index as u32 + 1;
        let object_id = bodies.len() as u32 + index as u32 + 1;
        materials.push_str(&format!(
            "<basematerials id=\"{mat}\"><base name=\"{}\" displaycolor=\"{}\"/></basematerials>",
            xml_escape(&body.name),
            body.color.hex_string()
        ));
        let mut vertices = String::new();
        for position in &body.mesh.positions {
            let p = to_z_up(*position);
            vertices.push_str(&format!(
                "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
                num(p[0]),
                num(p[1]),
                num(p[2])
            ));
        }
        let mut triangles = String::new();
        for tri in body.mesh.indices.chunks_exact(3) {
            triangles.push_str(&format!(
                "<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\" pid=\"{mat}\" p1=\"0\"/>",
                tri[0], tri[1], tri[2]
            ));
        }
        objects.push_str(&format!(
            "<object id=\"{object_id}\" name=\"{}\" type=\"model\" pid=\"{mat}\" pindex=\"0\"><mesh><vertices>{vertices}</vertices><triangles>{triangles}</triangles></mesh></object>",
            xml_escape(&body.name)
        ));
        build.push_str(&format!("<item objectid=\"{object_id}\"/>"));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" xmlns:m=\"http://schemas.microsoft.com/3dmanufacturing/material/2015/02\"><resources>{materials}{objects}</resources><build>{build}</build></model>"
    )
}

fn parse_3mf(text: &str) -> Result<Vec<Body>, ExchangeError> {
    let mut colors = std::collections::HashMap::<String, Color>::new();
    for tag in xml_tags(text, "base") {
        if let (Some(group), Some(color)) =
            (parent_material_id(text, &tag), attr(&tag, "displaycolor"))
        {
            colors.insert(group, parse_hex(&color));
        }
    }
    let mut bodies = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<object ") {
        let after = &rest[start..];
        let Some(end) = after.find("</object>") else {
            break;
        };
        let block = &after[..end];
        let name = attr(block, "name").unwrap_or_else(|| "3MF".into());
        let pid = attr(block, "pid").unwrap_or_default();
        let color = colors.get(&pid).copied().unwrap_or(DEFAULT_MODEL_COLOR);
        let mut positions = Vec::new();
        for tag in xml_tags(block, "vertex") {
            let x = attr_f32(&tag, "x")?;
            let y = attr_f32(&tag, "y")?;
            let z = attr_f32(&tag, "z")?;
            positions.push(to_y_up([x, y, z]));
        }
        let mut indices = Vec::new();
        for tag in xml_tags(block, "triangle") {
            indices.push(attr_u32(&tag, "v1")?);
            indices.push(attr_u32(&tag, "v2")?);
            indices.push(attr_u32(&tag, "v3")?);
        }
        guard_tris(indices.len() / 3)?;
        if positions.is_empty() || indices.is_empty() {
            return Err(ExchangeError::Malformed);
        }
        let normals = face_normals(&positions, &indices);
        bodies.push(Body {
            name,
            color,
            mesh: Mesh {
                positions,
                normals,
                indices,
            },
        });
        rest = &after[end + "</object>".len()..];
    }
    Ok(bodies)
}

fn parent_material_id(text: &str, base_tag: &str) -> Option<String> {
    let at = text.find(base_tag)?;
    let before = &text[..at];
    let open = before.rfind("<basematerials")?;
    attr(&before[open..], "id")
}

fn parse_obj(text: &str) -> Result<Vec<Body>, ExchangeError> {
    let mut verts = Vec::new();
    let mut colors = Vec::new();
    let mut ranges: Vec<(String, usize, Vec<[usize; 3]>)> = Vec::new();
    let mut current = "OBJ".to_string();
    let mut faces = Vec::new();
    let mut face_start = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else { continue };
        match kind {
            "o" | "g" => {
                if !faces.is_empty() {
                    ranges.push((current.clone(), face_start, std::mem::take(&mut faces)));
                    face_start = verts.len();
                }
                current = line[kind.len()..].trim().to_string();
                if current.is_empty() {
                    current = "OBJ".into();
                }
            }
            "v" => {
                let nums: Vec<f32> = parts.filter_map(|part| part.parse().ok()).collect();
                if nums.len() < 3 {
                    return Err(ExchangeError::Malformed);
                }
                verts.push(to_y_up([nums[0], nums[1], nums[2]]));
                colors.push(if nums.len() >= 6 {
                    Some(Color {
                        r: (nums[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                        g: (nums[4].clamp(0.0, 1.0) * 255.0).round() as u8,
                        b: (nums[5].clamp(0.0, 1.0) * 255.0).round() as u8,
                    })
                } else {
                    None
                });
            }
            "f" => {
                let idx: Vec<usize> = parts
                    .filter_map(|part| part.split('/').next().unwrap_or("").parse::<i32>().ok())
                    .filter(|index| *index != 0)
                    .map(|index| {
                        if index > 0 {
                            index as usize
                        } else {
                            verts.len() + 1 - index.unsigned_abs() as usize
                        }
                    })
                    .collect();
                if idx.len() < 3 {
                    return Err(ExchangeError::Malformed);
                }
                for window in 1..idx.len() - 1 {
                    faces.push([idx[0] - 1, idx[window] - 1, idx[window + 1] - 1]);
                }
            }
            _ => {}
        }
    }
    if !faces.is_empty() {
        ranges.push((current, face_start, faces));
    }
    if ranges.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    guard_tris(ranges.iter().map(|(_, _, faces)| faces.len()).sum())?;
    let mut bodies = Vec::new();
    for (name, _start, faces) in ranges {
        let mut used = Vec::new();
        let mut map = std::collections::HashMap::new();
        let mut indices = Vec::new();
        for face in faces {
            if face.iter().any(|index| *index >= verts.len()) {
                return Err(ExchangeError::Malformed);
            }
            for index in face {
                let slot = map.entry(index).or_insert_with(|| {
                    used.push(index);
                    used.len() - 1
                });
                indices.push(*slot as u32);
            }
        }
        let positions: Vec<[f32; 3]> = used.iter().map(|index| verts[*index]).collect();
        let color = used
            .iter()
            .find_map(|index| colors[*index])
            .unwrap_or(DEFAULT_MODEL_COLOR);
        let normals = face_normals(&positions, &indices);
        bodies.push(Body {
            name,
            color,
            mesh: Mesh {
                positions,
                normals,
                indices,
            },
        });
    }
    Ok(bodies)
}

fn svg_outlines(text: &str) -> Result<Vec<Vec<[f32; 2]>>, ExchangeError> {
    if !text.to_ascii_lowercase().contains("<svg") {
        return Err(ExchangeError::Malformed);
    }
    let mut outlines = Vec::new();
    for tag in xml_tags(text, "polygon") {
        if let Some(points) = attr(&tag, "points") {
            outlines.push(parse_pairs(&points));
        }
    }
    for tag in xml_tags(text, "polyline") {
        if let Some(points) = attr(&tag, "points") {
            let mut pts = parse_pairs(&points);
            if pts.len() >= 3 {
                close_loop(&mut pts);
                outlines.push(pts);
            }
        }
    }
    for tag in xml_tags(text, "rect") {
        let x = attr(&tag, "x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let y = attr(&tag, "y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let w = attr(&tag, "width")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        let h = attr(&tag, "height")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        if w > 0.0 && h > 0.0 {
            outlines.push(vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
        }
    }
    for tag in xml_tags(text, "path") {
        if let Some(data) = attr(&tag, "d") {
            if let Some(points) = path_lines(&data) {
                outlines.push(points);
            }
        }
    }
    let outlines: Vec<_> = outlines.into_iter().filter(|pts| pts.len() >= 3).collect();
    if outlines.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    Ok(outlines)
}

fn path_lines(data: &str) -> Option<Vec<[f32; 2]>> {
    let mut chars = data.chars().peekable();
    let mut cmd = 'M';
    let mut points = Vec::new();
    let mut current = [0.0f32, 0.0];
    let mut start = [0.0f32, 0.0];
    while chars.peek().is_some() {
        while matches!(chars.peek(), Some(c) if c.is_whitespace() || *c == ',') {
            chars.next();
        }
        if let Some(c) = chars.peek().copied() {
            if c.is_ascii_alphabetic() {
                cmd = chars.next().unwrap();
            }
        }
        let absolute = cmd.is_ascii_uppercase();
        match cmd.to_ascii_uppercase() {
            'M' | 'L' => {
                let x = path_number(&mut chars)?;
                let y = path_number(&mut chars)?;
                current = if absolute {
                    [x, y]
                } else {
                    [current[0] + x, current[1] + y]
                };
                if cmd.eq_ignore_ascii_case(&'M') {
                    start = current;
                }
                points.push(current);
                if cmd.eq_ignore_ascii_case(&'M') {
                    cmd = if absolute { 'L' } else { 'l' };
                }
            }
            'H' => {
                let x = path_number(&mut chars)?;
                current[0] = if absolute { x } else { current[0] + x };
                points.push(current);
            }
            'V' => {
                let y = path_number(&mut chars)?;
                current[1] = if absolute { y } else { current[1] + y };
                points.push(current);
            }
            'Z' => {
                points.push(start);
                current = start;
            }
            'C' => {
                let numbers = [
                    path_number(&mut chars)?,
                    path_number(&mut chars)?,
                    path_number(&mut chars)?,
                    path_number(&mut chars)?,
                    path_number(&mut chars)?,
                    path_number(&mut chars)?,
                ];
                let (c1, c2, end) = if absolute {
                    (
                        [numbers[0], numbers[1]],
                        [numbers[2], numbers[3]],
                        [numbers[4], numbers[5]],
                    )
                } else {
                    (
                        [current[0] + numbers[0], current[1] + numbers[1]],
                        [current[0] + numbers[2], current[1] + numbers[3]],
                        [current[0] + numbers[4], current[1] + numbers[5]],
                    )
                };
                for step in 1..=8 {
                    let t = step as f32 / 8.0;
                    points.push(cubic(current, c1, c2, end, t));
                }
                current = end;
            }
            _ => return None,
        }
    }
    close_loop(&mut points);
    (points.len() >= 3).then_some(points)
}

fn path_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f32> {
    while matches!(chars.peek(), Some(c) if c.is_whitespace() || *c == ',') {
        chars.next();
    }
    let mut token = String::new();
    if matches!(chars.peek(), Some('+') | Some('-')) {
        token.push(chars.next().unwrap());
    }
    let mut saw = false;
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit() || *c == '.') {
        saw = true;
        token.push(chars.next().unwrap());
    }
    if saw {
        token.parse().ok()
    } else {
        None
    }
}

fn cubic(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    [
        u * u * u * p0[0] + 3.0 * u * u * t * p1[0] + 3.0 * u * t * t * p2[0] + t * t * t * p3[0],
        u * u * u * p0[1] + 3.0 * u * u * t * p1[1] + 3.0 * u * t * t * p2[1] + t * t * t * p3[1],
    ]
}

fn dxf_outlines(text: &str) -> Result<Vec<Vec<[f32; 2]>>, ExchangeError> {
    if !text.contains("SECTION") && !text.contains("LWPOLYLINE") && !text.contains("LINE") {
        return Err(ExchangeError::Malformed);
    }
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut pairs = Vec::new();
    let mut index = 0;
    while index + 1 < lines.len() {
        if let Ok(code) = lines[index].parse::<i32>() {
            pairs.push((code, lines[index + 1].to_string()));
            index += 2;
        } else {
            index += 1;
        }
    }
    let mut scale = 1.0f32;
    let mut variable = String::new();
    let mut outlines = Vec::new();
    let mut entity = String::new();
    let mut poly = Vec::new();
    let mut closed = false;
    let mut x = None;
    let mut circle = None::<[f32; 2]>;
    let mut radius = None;
    let flush = |outlines: &mut Vec<Vec<[f32; 2]>>,
                 entity: &str,
                 poly: &mut Vec<[f32; 2]>,
                 closed: bool,
                 scale: f32,
                 circle: &mut Option<[f32; 2]>,
                 radius: &mut Option<f32>| {
        if entity == "LWPOLYLINE"
            && poly.len() >= 3
            && (closed || nearly(*poly.first().unwrap(), *poly.last().unwrap()))
        {
            close_loop(poly);
            outlines.push(poly.iter().map(|p| [p[0] * scale, p[1] * scale]).collect());
        }
        if entity == "CIRCLE" {
            if let (Some(center), Some(radius)) = (*circle, *radius) {
                let mut pts = Vec::new();
                for step in 0..24 {
                    let angle = step as f32 / 24.0 * std::f32::consts::TAU;
                    pts.push([
                        (center[0] + angle.cos() * radius) * scale,
                        (center[1] + angle.sin() * radius) * scale,
                    ]);
                }
                outlines.push(pts);
            }
        }
        poly.clear();
        *circle = None;
        *radius = None;
    };
    for (code, value) in &pairs {
        if *code == 9 {
            variable = value.clone();
        }
        if variable == "$INSUNITS" && *code == 70 {
            scale = match value.parse::<i32>().unwrap_or(4) {
                1 => 25.4,
                5 => 10.0,
                6 => 1000.0,
                _ => 1.0,
            };
            variable.clear();
        }
        if *code == 0 {
            flush(
                &mut outlines,
                &entity,
                &mut poly,
                closed,
                scale,
                &mut circle,
                &mut radius,
            );
            entity = value.clone();
            closed = false;
            continue;
        }
        match entity.as_str() {
            "LWPOLYLINE" => match code {
                10 => x = value.parse().ok(),
                20 => {
                    if let Some(px) = x.take() {
                        if let Ok(py) = value.parse::<f32>() {
                            poly.push([px, py]);
                        }
                    }
                }
                70 => closed = value.parse::<i32>().unwrap_or(0) & 1 == 1,
                _ => {}
            },
            "CIRCLE" => match code {
                10 => x = value.parse().ok(),
                20 => {
                    if let Some(px) = x.take() {
                        if let Ok(py) = value.parse() {
                            circle = Some([px, py]);
                        }
                    }
                }
                40 => radius = value.parse().ok(),
                _ => {}
            },
            _ => {}
        }
    }
    flush(
        &mut outlines,
        &entity,
        &mut poly,
        closed,
        scale,
        &mut circle,
        &mut radius,
    );
    if outlines.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    Ok(outlines)
}

fn step_mesh(text: &str) -> Result<Mesh, ExchangeError> {
    let mut points = std::collections::HashMap::<u32, [f32; 3]>::new();
    for (id, body) in step_entities(text) {
        if body.starts_with("CARTESIAN_POINT") {
            if let Some(coords) = tuple_floats(&body) {
                if coords.len() >= 3 {
                    points.insert(id, to_y_up([coords[0], coords[1], coords[2]]));
                }
            }
        }
    }
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for (_id, body) in step_entities(text) {
        if !body.starts_with("POLY_LOOP") {
            continue;
        }
        let refs = ref_list(&body);
        if refs.len() < 3 {
            continue;
        }
        let ids: Vec<u32> = refs
            .iter()
            .filter_map(|item| item.trim().trim_start_matches('#').parse().ok())
            .collect();
        if ids.len() < 3 || ids.iter().any(|id| !points.contains_key(id)) {
            return Err(ExchangeError::Malformed);
        }
        let base = positions.len() as u32;
        for id in ids.iter().take(3) {
            positions.push(points[id]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
        for extra in 3..ids.len() {
            let next = positions.len() as u32;
            positions.push(points[&ids[extra]]);
            indices.extend_from_slice(&[base, next - 1, next]);
        }
    }
    if indices.is_empty() {
        let upper = text.to_ascii_uppercase();
        if upper.contains("B_SPLINE")
            || upper.contains("ADVANCED_BREP")
            || upper.contains("MANIFOLD_SOLID_BREP")
            || upper.contains("ADVANCED_FACE")
        {
            return Err(ExchangeError::UnsupportedStep);
        }
        return Err(ExchangeError::Malformed);
    }
    guard_tris(indices.len() / 3)?;
    let normals = face_normals(&positions, &indices);
    Ok(Mesh {
        positions,
        normals,
        indices,
    })
}

fn step_entities(text: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let Ok(id) = line[1..eq].trim().parse::<u32>() else {
            continue;
        };
        let body = line[eq + 1..].trim().trim_end_matches(';').to_string();
        out.push((id, body));
    }
    out
}

fn tuple_floats(body: &str) -> Option<Vec<f32>> {
    let start = body.rfind('(')?;
    let end = body[start..].find(')')? + start;
    let inner = &body[start + 1..end];
    let nums: Vec<f32> = inner
        .split(',')
        .filter_map(|part| part.trim().parse().ok())
        .collect();
    (!nums.is_empty()).then_some(nums)
}

fn ref_list(body: &str) -> Vec<String> {
    let Some(start) = body.find("(#") else {
        return Vec::new();
    };
    let Some(end) = body[start..].find(')') else {
        return Vec::new();
    };
    body[start + 1..start + end]
        .split(',')
        .map(|item| item.trim().to_string())
        .collect()
}

pub(crate) fn face_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    for tri in indices.chunks_exact(3) {
        let a = Vec3::from_array(positions[tri[0] as usize]);
        let b = Vec3::from_array(positions[tri[1] as usize]);
        let c = Vec3::from_array(positions[tri[2] as usize]);
        let n = (b - a).cross(c - a);
        let n = if n.length_squared() < 1e-12 {
            Vec3::Y
        } else {
            n.normalize()
        };
        for index in tri {
            normals[*index as usize] = n.to_array();
        }
    }
    normals
}

fn parse_pairs(text: &str) -> Vec<[f32; 2]> {
    let nums: Vec<f32> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|part| part.parse().ok())
        .collect();
    nums.chunks_exact(2)
        .map(|pair| [pair[0], pair[1]])
        .collect()
}

fn close_loop(points: &mut Vec<[f32; 2]>) {
    if points.len() >= 2 && nearly(*points.first().unwrap(), *points.last().unwrap()) {
        points.pop();
    }
}

fn nearly(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
}

fn obj_color(color: Color) -> String {
    format!(
        "{:.4} {:.4} {:.4}",
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0
    )
}

fn num(value: f32) -> String {
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    if rounded == 0.0 {
        "0".into()
    } else {
        format!("{rounded}")
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn xml_tags<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let needle = format!("<{name}");
    let mut rest = text;
    while let Some(start) = rest.find(&needle) {
        let after = &rest[start..];
        let boundary = after.as_bytes().get(needle.len()).copied();
        if !matches!(
            boundary,
            Some(b' ') | Some(b'/') | Some(b'>') | Some(b'\n') | Some(b'\r')
        ) {
            rest = &after[needle.len()..];
            continue;
        }
        let Some(end) = after.find('>') else { break };
        out.push(&after[..=end]);
        rest = &after[end + 1..];
    }
    out
}

pub(crate) fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

pub(crate) fn attr_f32(tag: &str, name: &str) -> Result<f32, ExchangeError> {
    attr(tag, name)
        .and_then(|value| value.parse().ok())
        .ok_or(ExchangeError::Malformed)
}

pub(crate) fn attr_u32(tag: &str, name: &str) -> Result<u32, ExchangeError> {
    attr(tag, name)
        .and_then(|value| value.parse().ok())
        .ok_or(ExchangeError::Malformed)
}

fn parse_hex(text: &str) -> Color {
    let hex = text.trim().trim_start_matches('#');
    u32::from_str_radix(hex, 16)
        .map(Color::from_hex)
        .unwrap_or(DEFAULT_MODEL_COLOR)
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>"#;

pub(crate) fn zip_store(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let name_bytes = name.as_bytes();
        let crc = crc32(data);
        let local_at = out.len() as u32;
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&local_at.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let central_at = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&central_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

pub(crate) fn unzip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ExchangeError> {
    if bytes.len() < 22 || u32::from_le_bytes(bytes[0..4].try_into().unwrap()) != 0x04034b50 {
        return Err(ExchangeError::Malformed);
    }
    let mut files = Vec::new();
    let mut cursor = 0usize;
    while cursor + 30 <= bytes.len() {
        let sig = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        if sig != 0x04034b50 {
            break;
        }
        let method = u16::from_le_bytes(bytes[cursor + 8..cursor + 10].try_into().unwrap());
        let comp_size =
            u32::from_le_bytes(bytes[cursor + 18..cursor + 22].try_into().unwrap()) as usize;
        let name_len =
            u16::from_le_bytes(bytes[cursor + 26..cursor + 28].try_into().unwrap()) as usize;
        let extra_len =
            u16::from_le_bytes(bytes[cursor + 28..cursor + 30].try_into().unwrap()) as usize;
        let name_at = cursor + 30;
        if name_at + name_len + extra_len + comp_size > bytes.len() {
            return Err(ExchangeError::Malformed);
        }
        let name = String::from_utf8_lossy(&bytes[name_at..name_at + name_len]).to_string();
        let data =
            &bytes[name_at + name_len + extra_len..name_at + name_len + extra_len + comp_size];
        let plain = match method {
            0 => data.to_vec(),
            8 => {
                let mut decoder = flate2::read::DeflateDecoder::new(data);
                let mut plain = Vec::new();
                decoder
                    .read_to_end(&mut plain)
                    .map_err(|_| ExchangeError::Malformed)?;
                plain
            }
            _ => return Err(ExchangeError::Malformed),
        };
        files.push((name, plain));
        cursor = name_at + name_len + extra_len + comp_size;
    }
    if files.is_empty() {
        return Err(ExchangeError::Malformed);
    }
    Ok(files)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
