//! A contour slicer for the slicer page.
//!
//! This is not CuraEngine. It cuts each body with horizontal planes and writes wall loops,
//! grid, line, or concentric infill, and a skirt or brim. Supports, ironing, and the other
//! infill patterns are kept as settings and reported as not generated.

use glam::Vec3;

use crate::document::Document;
use crate::mesh::Mesh;

pub const BED_MM: f32 = 256.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetId {
    Draft,
    Standard,
    Fine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adhesion {
    Skirt,
    Brim,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfillPattern {
    Grid,
    Lines,
    Concentric,
    /// Stored, then sliced as grid.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    OuterWall,
    InnerWall,
    Skin,
    Infill,
    Skirt,
}

#[derive(Clone, Debug)]
pub struct Toolpath {
    pub kind: PathKind,
    /// Points in plate XZ. The path is closed when the first and last points match.
    pub points: Vec<[f32; 2]>,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct SliceLayer {
    pub z: f32,
    pub paths: Vec<Toolpath>,
}

#[derive(Clone, Debug)]
pub struct SliceSettings {
    pub layer_height: f32,
    pub first_layer_height: f32,
    pub walls: u32,
    pub top_layers: u32,
    pub bottom_layers: u32,
    pub infill: f32,
    pub infill_pattern: InfillPattern,
    pub nozzle: f32,
    pub adhesion: Adhesion,
    pub brim_width: f32,
    pub speed_outer: f32,
    pub speed_inner: f32,
    pub speed_infill: f32,
    pub speed_first: f32,
    pub speed_travel: f32,
    pub density: f32,
    pub nozzle_temp: Option<f32>,
    pub bed_temp: Option<f32>,
    pub supports: bool,
    pub support_style: &'static str,
}

#[derive(Clone, Debug)]
pub struct SliceResult {
    pub layers: Vec<SliceLayer>,
    pub filament_mm: f32,
    pub filament_g: f32,
    pub print_time_sec: f32,
    pub gcode: String,
    /// What this engine did not build.
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SliceError {
    Empty,
    TooBig(String),
    BadSettings(String),
}

impl std::fmt::Display for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliceError::Empty => f.write_str("There is nothing on this plate to slice."),
            SliceError::TooBig(text) => f.write_str(text),
            SliceError::BadSettings(text) => f.write_str(text),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MaterialTemps {
    pub kind: &'static str,
    pub nozzle: i32,
    pub bed: i32,
    pub density: f32,
    pub fan: i32,
}

pub fn material_temps(kind: Option<&str>) -> MaterialTemps {
    let key = kind.unwrap_or("PLA").to_ascii_uppercase();
    let table = [
        ("PLA", 220, 55, 1.24, 100),
        ("PETG", 250, 70, 1.27, 40),
        ("PET", 260, 70, 1.30, 30),
        ("ABS", 260, 100, 1.04, 10),
        ("ASA", 260, 100, 1.07, 10),
        ("TPU", 230, 35, 1.21, 60),
        ("PC", 270, 110, 1.20, 10),
        ("PA", 280, 100, 1.12, 0),
        ("PVA", 220, 55, 1.23, 50),
    ];
    let hit = table
        .iter()
        .find(|row| key == row.0)
        .or_else(|| table.iter().find(|row| key.starts_with(row.0)))
        .unwrap_or(&table[0]);
    MaterialTemps {
        kind: hit.0,
        nozzle: hit.1,
        bed: hit.2,
        density: hit.3,
        fan: hit.4,
    }
}

pub fn standard_settings() -> SliceSettings {
    SliceSettings {
        layer_height: 0.2,
        first_layer_height: 0.2,
        walls: 2,
        top_layers: 5,
        bottom_layers: 3,
        infill: 15.0,
        infill_pattern: InfillPattern::Grid,
        nozzle: 0.4,
        adhesion: Adhesion::Skirt,
        brim_width: 5.0,
        speed_outer: 200.0,
        speed_inner: 300.0,
        speed_infill: 270.0,
        speed_first: 50.0,
        speed_travel: 500.0,
        density: 1.24,
        nozzle_temp: None,
        bed_temp: None,
        supports: false,
        support_style: "tree",
    }
}

pub fn apply_preset(mut settings: SliceSettings, preset: PresetId) -> SliceSettings {
    let (height, walls, infill) = match preset {
        PresetId::Draft => (0.28, 2, 10.0),
        PresetId::Standard => (0.2, 2, 15.0),
        PresetId::Fine => (0.12, 3, 15.0),
    };
    settings.layer_height = height;
    settings.walls = walls;
    settings.infill = infill;
    settings.top_layers = (0.8 / height).ceil() as u32;
    settings.bottom_layers = (0.6 / height).ceil() as u32;
    settings
}

pub fn approximate_layer(height_mm: f32) -> u32 {
    (height_mm / 0.2).round().max(1.0) as u32
}

pub fn apply_bambu_process(settings: &mut SliceSettings, json: &str) -> Result<(), SliceError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| SliceError::BadSettings("That settings file is not JSON.".into()))?;
    let object = value.as_object().ok_or_else(|| {
        SliceError::BadSettings("That settings file is not a Bambu preset.".into())
    })?;
    if let Some(height) = number_of(object.get("layer_height")) {
        settings.layer_height = height.clamp(0.08, 0.6);
    }
    if let Some(walls) = number_of(object.get("wall_loops")) {
        settings.walls = walls.clamp(1.0, 20.0) as u32;
    }
    if let Some(infill) = number_of(object.get("sparse_infill_density")) {
        settings.infill = infill.clamp(0.0, 100.0);
    }
    if let Some(temp) = number_of(object.get("nozzle_temperature")) {
        settings.nozzle_temp = Some(temp.clamp(150.0, 350.0));
    }
    if let Some(temp) = number_of(object.get("textured_plate_temp")) {
        settings.bed_temp = Some(temp.clamp(0.0, 130.0));
    }
    if let Some(flag) = object.get("enable_support").and_then(|v| v.as_str()) {
        settings.supports = flag == "1" || flag.eq_ignore_ascii_case("true");
    }
    Ok(())
}

fn number_of(value: Option<&serde_json::Value>) -> Option<f32> {
    let text = match value {
        Some(serde_json::Value::String(text)) => text.trim().trim_end_matches('%'),
        Some(serde_json::Value::Number(number)) => return number.as_f64().map(|n| n as f32),
        _ => return None,
    };
    text.parse().ok()
}

#[derive(Clone, Debug)]
pub struct ParsedGcode {
    /// Segments in Y-up millimetres: printer `(x, y, z)` becomes `(x, z, -y)`.
    pub segments: Vec<([f32; 3], [f32; 3])>,
    pub layer_z: Vec<f32>,
    pub filament_mm: f32,
}

pub fn parse_gcode(text: &str) -> ParsedGcode {
    let mut segments = Vec::new();
    let mut layer_z = Vec::new();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut e = 0.0;
    let mut relative_e = false;
    let mut in_layer = false;
    let mut filament = 0.0;
    for raw in text.lines() {
        if raw.starts_with(";LAYER:") {
            if in_layer {
                layer_z.push(z);
            }
            in_layer = true;
            continue;
        }
        if raw.starts_with("M83") {
            relative_e = true;
            continue;
        }
        if raw.starts_with("M82") {
            relative_e = false;
            continue;
        }
        let is_move = raw.starts_with("G1 ") || raw.starts_with("G0 ");
        let is_reset = raw.starts_with("G92");
        if !is_move && !is_reset {
            continue;
        }
        let mut nx = x;
        let mut ny = y;
        let mut nz = z;
        let mut ne = None;
        for word in raw.split(';').next().unwrap_or(raw).split_whitespace() {
            let value = word[1..].parse().unwrap_or(0.0);
            match word.as_bytes().first() {
                Some(b'X') => nx = value,
                Some(b'Y') => ny = value,
                Some(b'Z') => nz = value,
                Some(b'E') => ne = Some(value),
                _ => {}
            }
        }
        if is_reset {
            if let Some(value) = ne {
                e = value;
            }
            continue;
        }
        let de = match ne {
            None => 0.0,
            Some(value) if relative_e => value,
            Some(value) => value - e,
        };
        if let Some(value) = ne {
            if !relative_e {
                e = value;
            }
        }
        if de > 0.0 {
            filament += de;
        }
        if in_layer && de > 0.0 && (nx != x || ny != y) {
            segments.push(([x, z, -y], [nx, nz, -ny]));
        }
        x = nx;
        y = ny;
        z = nz;
    }
    if in_layer {
        layer_z.push(z);
    }
    ParsedGcode {
        segments,
        layer_z,
        filament_mm: filament,
    }
}

pub fn slice_plate(
    doc: &Document,
    plate_number: u32,
    settings: &SliceSettings,
) -> Result<SliceResult, SliceError> {
    if settings.layer_height < 0.04 || settings.nozzle < 0.05 {
        return Err(SliceError::BadSettings(
            "Layer height and nozzle must be positive.".into(),
        ));
    }
    let meshes: Vec<Mesh> = doc
        .objects()
        .iter()
        .filter(|object| doc.is_shown(object.id))
        .filter(|object| doc.plate_at(object.position).number == plate_number)
        .map(|object| object.world_mesh())
        .filter(|mesh| mesh.triangle_count() > 0)
        .collect();
    if meshes.is_empty() {
        return Err(SliceError::Empty);
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for mesh in &meshes {
        let (lo, hi) = mesh.bounds();
        min = min.min(lo);
        max = max.max(hi);
    }
    let size = max - min;
    if size.x > BED_MM + 0.5 || size.z > BED_MM + 0.5 || size.y > BED_MM + 0.5 {
        return Err(SliceError::TooBig(format!(
            "The model ({:.0} × {:.0} × {:.0} mm) does not fit the {BED_MM:.0} × {BED_MM:.0} × {BED_MM:.0} mm bed.",
            size.x, size.z, size.y
        )));
    }
    let mut heights = Vec::new();
    let mut z = settings.first_layer_height.max(0.04);
    let top = max.y;
    while z <= top + 1e-3 && heights.len() < 4000 {
        heights.push(z);
        z += settings.layer_height;
    }
    if heights.is_empty() {
        return Err(SliceError::Empty);
    }
    let mut layers = Vec::new();
    let mut filament_mm = 0.0;
    let mut seconds = 0.0;
    let mut cursor = [0.0_f32, 0.0];
    let mut gcode = String::from("; Sliced in PrintCaddy. Perimeters and infill only; supports and ironing are not generated.\n");
    gcode.push_str("M82\n");
    let mut extruded = 0.0;
    for (index, height) in heights.iter().copied().enumerate() {
        let mut paths = Vec::new();
        for mesh in &meshes {
            let loops = loops_at(mesh, height);
            for loop_pts in loops {
                let mut rings = vec![loop_pts.clone()];
                let mut current = loop_pts;
                for wall in 1..settings.walls {
                    match inset(&current, settings.nozzle) {
                        Some(next) => {
                            rings.push(next.clone());
                            current = next;
                        }
                        None => break,
                    }
                    let _ = wall;
                }
                for (wall, ring) in rings.iter().enumerate() {
                    let kind = if wall == 0 {
                        PathKind::OuterWall
                    } else {
                        PathKind::InnerWall
                    };
                    paths.push(Toolpath {
                        kind,
                        points: closed(ring),
                        z: height,
                    });
                }
                let solid = index < settings.bottom_layers as usize
                    || index + settings.top_layers as usize >= heights.len();
                let density = if solid { 100.0 } else { settings.infill };
                let pattern = if solid {
                    InfillPattern::Lines
                } else {
                    settings.infill_pattern
                };
                let kind = if solid {
                    PathKind::Skin
                } else {
                    PathKind::Infill
                };
                paths.extend(infill_paths(
                    &current,
                    density,
                    settings.nozzle,
                    pattern,
                    kind,
                    height,
                ));
            }
        }
        if index == 0 {
            if let Some(skirt) = adhesion_path(&paths, settings, height) {
                paths.insert(0, skirt);
            }
        }
        gcode.push_str(&format!(";LAYER:{index}\n"));
        for path in &paths {
            let speed = if index == 0 {
                settings.speed_first
            } else {
                match path.kind {
                    PathKind::OuterWall | PathKind::Skirt => settings.speed_outer,
                    PathKind::InnerWall => settings.speed_inner,
                    PathKind::Skin | PathKind::Infill => settings.speed_infill,
                }
            };
            let layer_h = if index == 0 {
                settings.first_layer_height
            } else {
                settings.layer_height
            };
            let length = path_length(&path.points);
            seconds += length / speed.max(1.0);
            if let Some(first) = path.points.first() {
                let travel =
                    ((first[0] - cursor[0]).powi(2) + (first[1] - cursor[1]).powi(2)).sqrt();
                seconds += travel / settings.speed_travel.max(1.0);
            }
            let volume = length * layer_h * settings.nozzle;
            let de = volume / (std::f32::consts::PI * 0.875 * 0.875);
            filament_mm += de;
            extruded += de;
            gcode.push_str(&format!(";TYPE:{}\n", type_name(path.kind)));
            for (point_index, point) in path.points.iter().enumerate() {
                let prefix = if point_index == 0 { "G0" } else { "G1" };
                let extrude = if point_index == 0 {
                    String::new()
                } else {
                    format!(" E{extruded:.4}")
                };
                gcode.push_str(&format!(
                    "{prefix} X{:.3} Y{:.3} Z{:.3}{extrude}\n",
                    point[0], -point[1], path.z
                ));
            }
            if let Some(last) = path.points.last() {
                cursor = *last;
            }
        }
        layers.push(SliceLayer { z: height, paths });
    }
    let density = if settings.density > 0.0 {
        settings.density
    } else {
        material_temps(None).density
    };
    let filament_g = filament_mm * std::f32::consts::PI * 0.875 * 0.875 / 1000.0 * density;
    let mut note = String::from(
        "Perimeters, infill, and skirt or brim were generated. Support towers, tree supports, and ironing were not.",
    );
    if settings.supports {
        note.push_str(" Support is on in the settings and is named here only.");
    }
    if settings.infill_pattern == InfillPattern::Other {
        note.push_str(" This infill pattern was sliced as a grid.");
    }
    Ok(SliceResult {
        layers,
        filament_mm,
        filament_g,
        print_time_sec: seconds,
        gcode,
        note,
    })
}

fn type_name(kind: PathKind) -> &'static str {
    match kind {
        PathKind::OuterWall => "WALL-OUTER",
        PathKind::InnerWall => "WALL-INNER",
        PathKind::Skin => "SKIN",
        PathKind::Infill => "FILL",
        PathKind::Skirt => "SKIRT",
    }
}

fn loops_at(mesh: &Mesh, y: f32) -> Vec<Vec<[f32; 2]>> {
    let mut segments = Vec::new();
    for tri in mesh.indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let pts = [
            mesh.positions[tri[0] as usize],
            mesh.positions[tri[1] as usize],
            mesh.positions[tri[2] as usize],
        ];
        let mut hits = Vec::new();
        for edge in [(0, 1), (1, 2), (2, 0)] {
            if let Some(point) = cross(pts[edge.0], pts[edge.1], y) {
                if hits
                    .iter()
                    .all(|have: &[f32; 2]| (have[0] - point[0]).hypot(have[1] - point[1]) > 1e-3)
                {
                    hits.push(point);
                }
            }
        }
        if hits.len() == 2 {
            segments.push((hits[0], hits[1]));
        }
    }
    chain(&segments)
}

fn cross(a: [f32; 3], b: [f32; 3], y: f32) -> Option<[f32; 2]> {
    let dy = b[1] - a[1];
    if dy.abs() < 1e-8 {
        return None;
    }
    let t = (y - a[1]) / dy;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some([a[0] + (b[0] - a[0]) * t, a[2] + (b[2] - a[2]) * t])
}

fn chain(segments: &[([f32; 2], [f32; 2])]) -> Vec<Vec<[f32; 2]>> {
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut poly = vec![segments[start].0, segments[start].1];
        loop {
            let end = *poly.last().unwrap();
            let mut found = None;
            for (index, segment) in segments.iter().enumerate() {
                if used[index] {
                    continue;
                }
                if near(segment.0, end) {
                    found = Some((index, segment.1));
                    break;
                }
                if near(segment.1, end) {
                    found = Some((index, segment.0));
                    break;
                }
            }
            let Some((index, next)) = found else {
                break;
            };
            used[index] = true;
            if near(next, poly[0]) {
                break;
            }
            poly.push(next);
            if poly.len() > segments.len() + 2 {
                break;
            }
        }
        if poly.len() >= 3 && polygon_area(&poly).abs() > 0.5 {
            if polygon_area(&poly) < 0.0 {
                poly.reverse();
            }
            loops.push(poly);
        }
    }
    loops
}

fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 0.05
}

fn polygon_area(poly: &[[f32; 2]]) -> f32 {
    let mut sum = 0.0;
    for index in 0..poly.len() {
        let next = poly[(index + 1) % poly.len()];
        sum += poly[index][0] * next[1] - next[0] * poly[index][1];
    }
    sum * 0.5
}

fn inset(poly: &[[f32; 2]], dist: f32) -> Option<Vec<[f32; 2]>> {
    offset(poly, dist)
}

fn offset(poly: &[[f32; 2]], dist: f32) -> Option<Vec<[f32; 2]>> {
    if poly.len() < 3 || dist == 0.0 {
        return Some(poly.to_vec());
    }
    let n = poly.len();
    let mut lines = Vec::new();
    for index in 0..n {
        let a = poly[index];
        let b = poly[(index + 1) % n];
        let dx = b[0] - a[0];
        let dz = b[1] - a[1];
        let len = dx.hypot(dz);
        if len < 1e-5 {
            continue;
        }
        let nx = -dz / len;
        let nz = dx / len;
        lines.push((
            [a[0] + nx * dist, a[1] + nz * dist],
            [b[0] + nx * dist, b[1] + nz * dist],
        ));
    }
    if lines.len() < 3 {
        return None;
    }
    let mut out = Vec::new();
    for index in 0..lines.len() {
        let hit = line_hit(lines[index], lines[(index + 1) % lines.len()])?;
        out.push(hit);
    }
    if polygon_area(&out).abs() < 1.0 {
        return None;
    }
    Some(out)
}

fn line_hit(a: ([f32; 2], [f32; 2]), b: ([f32; 2], [f32; 2])) -> Option<[f32; 2]> {
    let (p, r) = (a.0, [a.1[0] - a.0[0], a.1[1] - a.0[1]]);
    let (q, s) = (b.0, [b.1[0] - b.0[0], b.1[1] - b.0[1]]);
    let denom = r[0] * s[1] - r[1] * s[0];
    if denom.abs() < 1e-8 {
        return None;
    }
    let t = ((q[0] - p[0]) * s[1] - (q[1] - p[1]) * s[0]) / denom;
    Some([p[0] + r[0] * t, p[1] + r[1] * t])
}

fn infill_paths(
    poly: &[[f32; 2]],
    density: f32,
    nozzle: f32,
    pattern: InfillPattern,
    kind: PathKind,
    z: f32,
) -> Vec<Toolpath> {
    if density <= 0.0 || poly.len() < 3 {
        return Vec::new();
    }
    if pattern == InfillPattern::Concentric {
        let mut paths = Vec::new();
        let step = (nozzle / (density / 100.0).clamp(0.05, 1.0)).clamp(nozzle, 8.0);
        let mut ring = poly.to_vec();
        for _ in 0..40 {
            match offset(&ring, step) {
                Some(next) => {
                    paths.push(Toolpath {
                        kind,
                        points: closed(&next),
                        z,
                    });
                    ring = next;
                }
                None => break,
            }
        }
        return paths;
    }
    let spacing = (nozzle / (density / 100.0).clamp(0.05, 1.0)).clamp(nozzle, 20.0);
    let (min_x, max_x, min_z, max_z) = aabb(poly);
    let mut paths = Vec::new();
    let mut z_cursor = min_z + spacing * 0.5;
    while z_cursor < max_z {
        for span in clip_horizontal(poly, z_cursor) {
            paths.push(Toolpath {
                kind,
                points: vec![[span.0, z_cursor], [span.1, z_cursor]],
                z,
            });
        }
        z_cursor += spacing;
    }
    if matches!(pattern, InfillPattern::Grid | InfillPattern::Other) {
        let mut x_cursor = min_x + spacing * 0.5;
        while x_cursor < max_x {
            for span in clip_vertical(poly, x_cursor) {
                paths.push(Toolpath {
                    kind,
                    points: vec![[x_cursor, span.0], [x_cursor, span.1]],
                    z,
                });
            }
            x_cursor += spacing;
        }
    }
    paths
}

fn adhesion_path(paths: &[Toolpath], settings: &SliceSettings, z: f32) -> Option<Toolpath> {
    let points: Vec<[f32; 2]> = paths
        .iter()
        .filter(|path| path.kind == PathKind::OuterWall)
        .flat_map(|path| path.points.iter().copied())
        .collect();
    if points.is_empty() {
        return None;
    }
    let (min_x, max_x, min_z, max_z) = aabb(&points);
    match settings.adhesion {
        Adhesion::None => None,
        Adhesion::Skirt => Some(Toolpath {
            kind: PathKind::Skirt,
            points: rect(min_x - 3.0, max_x + 3.0, min_z - 3.0, max_z + 3.0),
            z,
        }),
        Adhesion::Brim => {
            let extra = settings.brim_width.max(settings.nozzle);
            Some(Toolpath {
                kind: PathKind::Skirt,
                points: rect(min_x - extra, max_x + extra, min_z - extra, max_z + extra),
                z,
            })
        }
    }
}

fn rect(min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> Vec<[f32; 2]> {
    vec![
        [min_x, min_z],
        [max_x, min_z],
        [max_x, max_z],
        [min_x, max_z],
        [min_x, min_z],
    ]
}

fn closed(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut out = points.to_vec();
    if let (Some(first), Some(last)) = (out.first().copied(), out.last().copied()) {
        if !near(first, last) {
            out.push(first);
        }
    }
    out
}

fn path_length(points: &[[f32; 2]]) -> f32 {
    points
        .windows(2)
        .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]))
        .sum()
}

fn aabb(points: &[[f32; 2]]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    for point in points {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_z = min_z.min(point[1]);
        max_z = max_z.max(point[1]);
    }
    (min_x, max_x, min_z, max_z)
}

fn clip_horizontal(poly: &[[f32; 2]], z: f32) -> Vec<(f32, f32)> {
    let mut xs = Vec::new();
    for index in 0..poly.len() {
        let a = poly[index];
        let b = poly[(index + 1) % poly.len()];
        if (a[1] - z) * (b[1] - z) > 0.0 || (a[1] - b[1]).abs() < 1e-8 {
            continue;
        }
        let t = (z - a[1]) / (b[1] - a[1]);
        if (0.0..=1.0).contains(&t) {
            xs.push(a[0] + (b[0] - a[0]) * t);
        }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs.chunks(2)
        .filter(|pair| pair.len() == 2 && pair[1] - pair[0] > 0.2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}

fn clip_vertical(poly: &[[f32; 2]], x: f32) -> Vec<(f32, f32)> {
    let mut zs = Vec::new();
    for index in 0..poly.len() {
        let a = poly[index];
        let b = poly[(index + 1) % poly.len()];
        if (a[0] - x) * (b[0] - x) > 0.0 || (a[0] - b[0]).abs() < 1e-8 {
            continue;
        }
        let t = (x - a[0]) / (b[0] - a[0]);
        if (0.0..=1.0).contains(&t) {
            zs.push(a[1] + (b[1] - a[1]) * t);
        }
    }
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    zs.chunks(2)
        .filter(|pair| pair.len() == 2 && pair[1] - pair[0] > 0.2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}
