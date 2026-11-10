//! Plate sketches: a drawn outline extruded, revolved, or swept.
//!
//! The sketch plane uses `(u, v)`. On a fresh plate sketch `u` is X and `v` is −Z, so a click at
//! world `(x, z)` is stored as `(x, −z)` and an extrude grows along +Y. Revolve spins around `v`.

use glam::Vec3;

use crate::mesh::Mesh;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SketchOp {
    Extrude,
    Revolve,
    Sweep,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SketchDef {
    pub profile: Vec<[f32; 2]>,
    pub path: Vec<[f32; 2]>,
    pub op: SketchOp,
    pub height: f32,
    pub angle_deg: f32,
    pub origin: [f32; 3],
    pub axis_u: [f32; 3],
    pub axis_v: [f32; 3],
}

impl SketchDef {
    pub fn on_plate(
        op: SketchOp,
        profile: Vec<[f32; 2]>,
        path: Vec<[f32; 2]>,
        height: f32,
        angle_deg: f32,
    ) -> Self {
        Self {
            profile,
            path,
            op,
            height,
            angle_deg,
            origin: [0.0, 0.0, 0.0],
            axis_u: [1.0, 0.0, 0.0],
            // v points along -Z so u × v is +Y and an extrude grows up off the plate.
            axis_v: [0.0, 0.0, -1.0],
        }
    }

    pub fn normal(&self) -> Vec3 {
        Vec3::from(self.axis_u)
            .cross(Vec3::from(self.axis_v))
            .normalize_or_zero()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SketchError {
    TooFewProfilePoints,
    ZeroArea,
    DuplicatePoint,
    SelfIntersecting,
    CrossesAxis,
    NoRadius,
    TooFewPathPoints,
    ShortPath,
    DegenerateSweep,
}

impl std::fmt::Display for SketchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            SketchError::TooFewProfilePoints => "The profile needs at least three points.",
            SketchError::ZeroArea => "The profile has no area.",
            SketchError::DuplicatePoint => "Two profile points are on top of each other.",
            SketchError::SelfIntersecting => "The outline crosses itself.",
            SketchError::CrossesAxis => "The profile crosses the revolve axis.",
            SketchError::NoRadius => "The profile has no radius from the revolve axis.",
            SketchError::TooFewPathPoints => "The path needs at least two points.",
            SketchError::ShortPath => "The path is too short.",
            SketchError::DegenerateSweep => "The profile cannot be swept along this path.",
        };
        f.write_str(text)
    }
}

/// Build the solid and shift it so its bottom-centre is the local origin.
/// The returned position is where that origin sits in the sketch's current coordinates.
pub fn seat_sketch(def: SketchDef) -> Result<(SketchDef, Vec3), SketchError> {
    let mesh = build_sketch(&def)?;
    let (min, max) = mesh.bounds();
    let center_x = (min.x + max.x) * 0.5;
    let center_z = (min.z + max.z) * 0.5;
    let shift = Vec3::new(-center_x, -min.y, -center_z);
    let mut seated = def;
    seated.origin = (Vec3::from(seated.origin) + shift).to_array();
    Ok((seated, Vec3::new(center_x, min.y, center_z)))
}

pub fn build_sketch(def: &SketchDef) -> Result<Mesh, SketchError> {
    let profile = cleaned_profile(&def.profile)?;
    match def.op {
        SketchOp::Extrude => {
            if def.height.abs() < 1e-3 {
                return Err(SketchError::ZeroArea);
            }
            Ok(extrude(def, &profile))
        }
        SketchOp::Revolve => {
            validate_revolve(&profile)?;
            let angle = def.angle_deg.abs().clamp(1.0, 360.0);
            Ok(revolve(def, &profile, angle))
        }
        SketchOp::Sweep => {
            validate_path(&def.path)?;
            Ok(sweep(def, &profile)?)
        }
    }
}

fn cleaned_profile(points: &[[f32; 2]]) -> Result<Vec<[f32; 2]>, SketchError> {
    let mut profile: Vec<[f32; 2]> = points
        .iter()
        .copied()
        .filter(|p| p[0].is_finite() && p[1].is_finite())
        .collect();
    if profile.len() >= 2 && dist2(profile[0], *profile.last().unwrap()) < 1e-6 {
        profile.pop();
    }
    if profile.len() < 3 {
        return Err(SketchError::TooFewProfilePoints);
    }
    for i in 0..profile.len() {
        for j in (i + 1)..profile.len() {
            if dist2(profile[i], profile[j]) < 1e-6 {
                return Err(SketchError::DuplicatePoint);
            }
        }
    }
    if signed_area(&profile).abs() < 1e-3 {
        return Err(SketchError::ZeroArea);
    }
    if is_self_intersecting(&profile) {
        return Err(SketchError::SelfIntersecting);
    }
    if signed_area(&profile) < 0.0 {
        profile.reverse();
    }
    Ok(profile)
}

fn validate_revolve(profile: &[[f32; 2]]) -> Result<(), SketchError> {
    let min_u = profile.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_u = profile
        .iter()
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max);
    if min_u < -1e-3 && max_u > 1e-3 {
        return Err(SketchError::CrossesAxis);
    }
    if max_u.abs().max(min_u.abs()) < 1e-3 {
        return Err(SketchError::NoRadius);
    }
    Ok(())
}

fn validate_path(path: &[[f32; 2]]) -> Result<(), SketchError> {
    if path.len() < 2 {
        return Err(SketchError::TooFewPathPoints);
    }
    let mut length = 0.0;
    for pair in path.windows(2) {
        length += dist2(pair[0], pair[1]).sqrt();
    }
    if length < 0.5 {
        return Err(SketchError::ShortPath);
    }
    Ok(())
}

fn extrude(def: &SketchDef, profile: &[[f32; 2]]) -> Mesh {
    let tris = triangulate(profile);
    let mut mesh = Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };
    let n = profile.len();
    let height = def.height;
    for tri in &tris {
        push_tri(
            &mut mesh,
            at(def, profile[tri[0]], 0.0),
            at(def, profile[tri[2]], 0.0),
            at(def, profile[tri[1]], 0.0),
        );
        push_tri(
            &mut mesh,
            at(def, profile[tri[0]], height),
            at(def, profile[tri[1]], height),
            at(def, profile[tri[2]], height),
        );
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let a = at(def, profile[i], 0.0);
        let b = at(def, profile[j], 0.0);
        let c = at(def, profile[j], height);
        let d = at(def, profile[i], height);
        push_tri(&mut mesh, a, b, c);
        push_tri(&mut mesh, a, c, d);
    }
    mesh
}

fn revolve(def: &SketchDef, profile: &[[f32; 2]], angle_deg: f32) -> Mesh {
    let steps = ((angle_deg / 360.0) * 32.0).round().max(3.0) as usize;
    let full = angle_deg > 359.0;
    let mut mesh = Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };
    let ring = |step: usize| -> Vec<[f32; 3]> {
        let t = if full {
            (step as f32) / (steps as f32)
        } else {
            (step as f32) / (steps as f32)
        };
        let theta = t * angle_deg.to_radians();
        profile
            .iter()
            .map(|p| revolve_point(def, *p, theta))
            .collect()
    };
    let turns = if full { steps } else { steps };
    for s in 0..turns {
        let a = ring(s);
        let b = ring(if full { (s + 1) % steps } else { s + 1 });
        for i in 0..profile.len() {
            let j = (i + 1) % profile.len();
            push_tri(&mut mesh, a[i], b[i], b[j]);
            push_tri(&mut mesh, a[i], b[j], a[j]);
        }
    }
    if !full {
        let start = ring(0);
        let end = ring(steps);
        cap_ring(&mut mesh, profile, &start, true);
        cap_ring(&mut mesh, profile, &end, false);
    }
    mesh
}

fn sweep(def: &SketchDef, profile: &[[f32; 2]]) -> Result<Mesh, SketchError> {
    let frames = path_frames(def)?;
    let mut mesh = Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };
    let rings: Vec<Vec<[f32; 3]>> = frames
        .iter()
        .map(|(point, normal, binormal)| {
            profile
                .iter()
                .map(|p| (*point + *binormal * p[0] + *normal * p[1]).to_array())
                .collect()
        })
        .collect();
    for s in 0..rings.len() - 1 {
        for i in 0..profile.len() {
            let j = (i + 1) % profile.len();
            push_tri(&mut mesh, rings[s][i], rings[s + 1][i], rings[s + 1][j]);
            push_tri(&mut mesh, rings[s][i], rings[s + 1][j], rings[s][j]);
        }
    }
    cap_ring(&mut mesh, profile, &rings[0], true);
    cap_ring(&mut mesh, profile, rings.last().unwrap(), false);
    Ok(mesh)
}

fn path_frames(def: &SketchDef) -> Result<Vec<(Vec3, Vec3, Vec3)>, SketchError> {
    let points: Vec<Vec3> = def.path.iter().map(|p| point_on_plane(def, *p)).collect();
    let mut frames = Vec::new();
    for i in 0..points.len() {
        let prev = if i == 0 { points[0] } else { points[i - 1] };
        let next = if i + 1 == points.len() {
            points[i]
        } else {
            points[i + 1]
        };
        let mut tangent = next - prev;
        if tangent.length_squared() < 1e-8 {
            return Err(SketchError::DegenerateSweep);
        }
        tangent = tangent.normalize();
        let up = def.normal();
        let mut normal = up - tangent * up.dot(tangent);
        if normal.length_squared() < 1e-6 {
            normal = Vec3::from(def.axis_u) - tangent * Vec3::from(def.axis_u).dot(tangent);
        }
        if normal.length_squared() < 1e-6 {
            return Err(SketchError::DegenerateSweep);
        }
        normal = normal.normalize();
        let binormal = tangent.cross(normal).normalize();
        if binormal.length_squared() < 0.5 {
            return Err(SketchError::DegenerateSweep);
        }
        frames.push((points[i], normal, binormal));
    }
    Ok(frames)
}

fn cap_ring(mesh: &mut Mesh, profile: &[[f32; 2]], ring: &[[f32; 3]], flip: bool) {
    for tri in triangulate(profile) {
        let (a, b, c) = if flip {
            (tri[0], tri[2], tri[1])
        } else {
            (tri[0], tri[1], tri[2])
        };
        push_tri(mesh, ring[a], ring[b], ring[c]);
    }
}

fn revolve_point(def: &SketchDef, uv: [f32; 2], theta: f32) -> [f32; 3] {
    let u = Vec3::from(def.axis_u);
    let v = Vec3::from(def.axis_v);
    let n = def.normal();
    let radial = u * uv[0];
    let spun = radial * theta.cos() + n * (uv[0] * theta.sin());
    (Vec3::from(def.origin) + spun + v * uv[1]).to_array()
}

fn at(def: &SketchDef, uv: [f32; 2], along: f32) -> [f32; 3] {
    (point_on_plane(def, uv) + def.normal() * along).to_array()
}

fn point_on_plane(def: &SketchDef, uv: [f32; 2]) -> Vec3 {
    Vec3::from(def.origin) + Vec3::from(def.axis_u) * uv[0] + Vec3::from(def.axis_v) * uv[1]
}

fn push_tri(mesh: &mut Mesh, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
    let n = face_normal(a, b, c);
    let base = mesh.positions.len() as u32;
    for p in [a, b, c] {
        mesh.positions.push(p);
        mesh.normals.push(n);
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = Vec3::from(b) - Vec3::from(a);
    let v = Vec3::from(c) - Vec3::from(a);
    let n = u.cross(v);
    if n.length_squared() < 1e-12 {
        [0.0, 1.0, 0.0]
    } else {
        n.normalize().to_array()
    }
}

fn triangulate(profile: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let n = profile.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }
    let mut left: Vec<usize> = (0..n).collect();
    let mut tris = Vec::new();
    let mut guard = 0;
    while left.len() > 3 && guard < n * n {
        guard += 1;
        let mut clipped = false;
        for i in 0..left.len() {
            let prev = left[(i + left.len() - 1) % left.len()];
            let curr = left[i];
            let next = left[(i + 1) % left.len()];
            if !is_ear(profile, &left, prev, curr, next) {
                continue;
            }
            tris.push([prev, curr, next]);
            left.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if left.len() == 3 {
        tris.push([left[0], left[1], left[2]]);
    }
    tris
}

fn is_ear(profile: &[[f32; 2]], left: &[usize], prev: usize, curr: usize, next: usize) -> bool {
    if cross(profile[prev], profile[curr], profile[next]) <= 1e-8 {
        return false;
    }
    for index in left {
        if *index == prev || *index == curr || *index == next {
            continue;
        }
        if point_in_tri(profile[*index], profile[prev], profile[curr], profile[next]) {
            return false;
        }
    }
    true
}

fn is_self_intersecting(profile: &[[f32; 2]]) -> bool {
    let n = profile.len();
    for i in 0..n {
        let a = profile[i];
        let b = profile[(i + 1) % n];
        for j in (i + 1)..n {
            let c = profile[j];
            let d = profile[(j + 1) % n];
            if (i + 1) % n == j || (j + 1) % n == i {
                continue;
            }
            if proper_cross(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

fn proper_cross(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    let o1 = cross(a, b, c);
    let o2 = cross(a, b, d);
    let o3 = cross(c, d, a);
    let o4 = cross(c, d, b);
    o1 * o2 < -1e-8 && o3 * o4 < -1e-8
}

fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn signed_area(profile: &[[f32; 2]]) -> f32 {
    let n = profile.len();
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += profile[i][0] * profile[j][1] - profile[j][0] * profile[i][1];
    }
    area * 0.5
}

fn point_in_tri(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let c1 = cross(a, b, p);
    let c2 = cross(b, c, p);
    let c3 = cross(c, a, p);
    (c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6) || (c1 <= 1e-6 && c2 <= 1e-6 && c3 <= 1e-6)
}

fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

/// Every edge of a closed solid is shared by two triangles.
pub fn is_closed(mesh: &Mesh) -> bool {
    use std::collections::HashMap;
    let mut edges: HashMap<([i32; 3], [i32; 3]), usize> = HashMap::new();
    let key = |p: [f32; 3]| {
        [
            (p[0] * 1000.0).round() as i32,
            (p[1] * 1000.0).round() as i32,
            (p[2] * 1000.0).round() as i32,
        ]
    };
    for tri in mesh.indices.chunks_exact(3) {
        let pts = [
            mesh.positions[tri[0] as usize],
            mesh.positions[tri[1] as usize],
            mesh.positions[tri[2] as usize],
        ];
        for i in 0..3 {
            let mut a = key(pts[i]);
            let mut b = key(pts[(i + 1) % 3]);
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            *edges.entry((a, b)).or_insert(0) += 1;
        }
    }
    edges.values().all(|count| *count == 2)
}
